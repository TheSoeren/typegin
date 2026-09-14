//! Click-to-walk pathfinding over a room's authored floor layout, the
//! standard adventure-game approach (Wintermute, Visionaire, AGS's modern
//! pathfinder): a room's walkable area is a polygon, and a path between two
//! points is the shortest route through the visibility graph of that
//! polygon's vertices (any two vertices — plus the start/end points — with
//! an unobstructed line of sight between them are connected, then shortest
//! path is Dijkstra/A* over that graph). This first increment covers only a
//! polygon's outer boundary — no interior obstacle holes yet (see AGENTS.md
//! discussion; holes are a deliberate follow-up, not deferred by oversight).
//!
//! Like [`crate::hotspot`]/[`crate::asset`], a room's walkable area is
//! authored data (`extra.gui.walkable`, a flat list of `[x, y]` boundary
//! points in room-local coordinates, same space as `gui.hotspot`), read by
//! [`walkable_area`] — `render` supplies the algorithm, a specific game
//! supplies the room geometry.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;

use typegin_core::ExtraValue;

use crate::extra::{as_array, as_number, as_table};

/// A room's walkable floor, as a single polygon boundary (room-local
/// coordinates, vertices in order — no interior holes in this increment).
#[derive(Debug, Clone, PartialEq)]
pub struct WalkableArea {
    pub boundary: Vec<(f32, f32)>,
}

/// Find the shortest walkable path from `start` to `goal` inside `area`, as
/// a sequence of waypoints to walk through in order (not including `start`
/// itself, since a caller already knows where it's standing; `goal` is
/// always the last waypoint when a path exists). A direct, unobstructed
/// line of sight yields a single-waypoint path (just `goal`); a concave
/// boundary may force the path through one or more of the polygon's own
/// vertices to stay inside the walkable area.
///
/// Returns `None` if `start` or `goal` lies outside `area` — clamping an
/// out-of-bounds click to the nearest walkable point is a caller concern
/// (or a later increment), not this function's.
#[must_use]
pub fn find_path(
    area: &WalkableArea,
    start: (f32, f32),
    goal: (f32, f32),
) -> Option<Vec<(f32, f32)>> {
    if !point_in_polygon(start, &area.boundary) || !point_in_polygon(goal, &area.boundary) {
        return None;
    }
    let (nodes, start_index, goal_index) = visibility_nodes(&area.boundary, start, goal);
    let graph = build_visibility_graph(&nodes, &area.boundary);

    dijkstra_evaluation(&graph, &nodes, start_index, goal_index)
}

/// One entry in the priority queue: a candidate distance to reach `node`.
/// `BinaryHeap` is a max-heap, so [`Ord`] is implemented *reversed*
/// (`other` compared against `self`) to make the *smallest* distance pop
/// first — the standard trick for using it as a min-heap.
#[derive(Debug, Clone, Copy)]
struct HeapEntry {
    distance: f32,
    node: usize,
}

// `PartialEq`/`Eq` are implemented manually (distance-only) to stay
// consistent with the `Ord`/`PartialOrd` impls below — a derived
// `PartialEq` would compare both fields, which could disagree with
// `Ord`'s distance-only comparison (e.g. two entries with equal distance
// but different nodes would be `Ord`-equal but derived-`PartialEq`-unequal,
// violating the usual "`Ord` agrees with `PartialEq`" expectation).
impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.distance == other.distance
    }
}

impl Eq for HeapEntry {}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Every distance here comes from `distance()` (an `f32::hypot` of
        // finite differences) added to a running total that starts at
        // `0.0`, so it's always finite — `partial_cmp` never sees a `NaN`.
        other.distance.partial_cmp(&self.distance).unwrap()
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Run Dijkstra over `graph` from `start` to `goal` (both node indices into
/// `nodes`), and reconstruct the shortest path as actual points — or `None`
/// if `goal` was never reached (no visibility-graph route exists between
/// them, which shouldn't happen for two points already confirmed inside a
/// hole-free polygon, but is handled rather than assumed away).
fn dijkstra_evaluation(
    graph: &HashMap<usize, Vec<usize>>,
    nodes: &[(f32, f32)],
    start: usize,
    goal: usize,
) -> Option<Vec<(f32, f32)>> {
    let mut visited: HashSet<usize> = HashSet::new();
    let mut dijkstra_table: HashMap<usize, f32> = HashMap::new();
    let mut predecessors: HashMap<usize, usize> = HashMap::new();
    dijkstra_table.insert(start, 0.0);

    let mut queue = BinaryHeap::new();
    queue.push(HeapEntry {
        distance: 0.0,
        node: start,
    });

    while let Some(HeapEntry {
        distance: current_distance,
        node: current_node,
    }) = queue.pop()
    {
        if !visited.insert(current_node) {
            continue;
        }
        if current_node == goal {
            break;
        }

        let Some(neighbors) = graph.get(&current_node) else {
            continue;
        };
        neighbors
            .iter()
            .filter(|&&neighbor| !visited.contains(&neighbor))
            .for_each(|&neighbor| {
                let candidate_distance =
                    current_distance + distance(nodes[current_node], nodes[neighbor]);
                let existing_distance = dijkstra_table
                    .get(&neighbor)
                    .copied()
                    .unwrap_or(f32::INFINITY);
                if candidate_distance < existing_distance {
                    dijkstra_table.insert(neighbor, candidate_distance);
                    predecessors.insert(neighbor, current_node);
                    queue.push(HeapEntry {
                        distance: candidate_distance,
                        node: neighbor,
                    });
                }
            });
    }

    if goal != start && !predecessors.contains_key(&goal) {
        return None;
    }

    // Walk `predecessors` backward from `goal` to `start`, then reverse —
    // `find_path`'s contract excludes `start` from the returned waypoints
    // (the loop condition `current != start` naturally stops before
    // pushing it), and always ends on `goal`.
    let mut path_indices = Vec::new();
    let mut current = goal;
    while current != start {
        path_indices.push(current);
        current = predecessors[&current];
    }
    path_indices.reverse();

    Some(path_indices.into_iter().map(|index| nodes[index]).collect())
}

/// Straight-line (Euclidean) distance between two points — the visibility
/// graph's edge weight, since a path's real cost is the physical distance
/// walked, not the number of hops between vertices.
fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The visibility graph as an adjacency list: each node's index maps to the
/// indices of every other node it has an unobstructed line of sight to,
/// tested against `boundary` — the polygon `nodes` was built from (see
/// `visibility_nodes`). Takes `boundary` as its own parameter, separate
/// from `nodes`, since `has_line_of_sight` needs to test candidate segments
/// against the polygon's actual edges, not against every other graph node
/// (which include `start`/`goal`, not real polygon edges).
///
/// An adjacency list (rather than a flat list of edges) is what lets
/// Dijkstra look up "this node's neighbors" in O(1) instead of scanning
/// every edge, and keeps the whole graph working by index — matching
/// `visited`/`start_index`/`goal_index` in `dijkstra_evaluation` — instead
/// of needing to hash or compare `(f32, f32)` points directly.
///
/// `i + 1..nodes.len()` (rather than `0..nodes.len()`) considers each
/// unordered pair exactly once — visibility is symmetric, so testing both
/// `(i, j)` and `(j, i)` would just repeat the same check. Since it's
/// symmetric, though, a discovered edge is inserted into *both* directions'
/// neighbor lists — otherwise Dijkstra could only ever walk it one way.
fn build_visibility_graph(
    nodes: &[(f32, f32)],
    boundary: &[(f32, f32)],
) -> HashMap<usize, Vec<usize>> {
    let mut graph: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            if has_line_of_sight(nodes[i], nodes[j], boundary) {
                graph.entry(i).or_default().push(j);
                graph.entry(j).or_default().push(i);
            }
        }
    }
    graph
}

/// Whether the segment `a1-a2` has an unobstructed line of sight inside the
/// polygon described by `boundary` — i.e. it stays entirely inside, rather
/// than cutting across the exterior (e.g. through a concave notch).
///
/// This is the practical visibility test (midpoint-inside plus no proper
/// edge crossing), not the fully rigorous local-vertex-angle visibility
/// test from computational geometry — a deliberate choice for a typical
/// room floor plan, but one that could in principle misjudge visibility on
/// extremely thin polygon slivers or near-tangent segments.
fn has_line_of_sight(a1: (f32, f32), a2: (f32, f32), boundary: &[(f32, f32)]) -> bool {
    // A segment that never crosses the boundary is either entirely inside
    // or entirely outside the (simple) polygon; checking one interior
    // sample point — the midpoint — is enough to tell which.
    let midpoint = (f32::midpoint(a1.0, a2.0), f32::midpoint(a1.1, a2.1));
    if !point_in_polygon(midpoint, boundary) {
        return false;
    }

    boundary_edges(boundary).all(|(b1, b2)| !segments_intersect(a1, a2, b1, b2))
}

/// Every edge of `boundary` as a `(vertex, previous vertex)` pair, including
/// the wraparound edge connecting the last vertex back to the first. Shared
/// by [`point_in_polygon`] and [`has_line_of_sight`], which both need to
/// walk a polygon's edges the same way — this indexing (`i` paired with
/// `i`'s predecessor, wrapping via `% len`) is exactly the kind of thing
/// that's easy to get subtly wrong twice.
fn boundary_edges(boundary: &[(f32, f32)]) -> impl Iterator<Item = ((f32, f32), (f32, f32))> + '_ {
    let len = boundary.len();
    (0..len).map(move |i| (boundary[i], boundary[(i + len - 1) % len]))
}

/// Sign of the cross product `(b - a) x (c - a)`: positive when `c` is a
/// counterclockwise turn from `a -> b`, negative when clockwise, zero when
/// `a`, `b`, `c` are collinear (this includes `c` being one of `a`/`b`
/// themselves, since a point has zero cross product with itself).
fn orientation(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

/// Whether segment `a1-a2` properly crosses segment `b1-b2` — i.e. each
/// segment's endpoints fall on strictly opposite sides of the other
/// segment's line. Requiring *strict*, nonzero opposite signs (rather than
/// just "different") is deliberate: it's what keeps two segments that only
/// touch at a shared endpoint (collinear, orientation zero) from counting
/// as crossing — exactly the case a visibility-graph candidate segment hits
/// constantly, since it starts/ends at the very vertices whose adjacent
/// boundary edges it's being tested against.
fn segments_intersect(a1: (f32, f32), a2: (f32, f32), b1: (f32, f32), b2: (f32, f32)) -> bool {
    let d1 = orientation(a1, a2, b1);
    let d2 = orientation(a1, a2, b2);
    let d3 = orientation(b1, b2, a1);
    let d4 = orientation(b1, b2, a2);

    ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
}

/// <a src="https://wrfranklin.org/Research/Short_Notes/pnpoly.html">pnpoly</a> algorithm to determine
/// if a point is inside the given polygon
fn point_in_polygon(point: (f32, f32), boundary: &[(f32, f32)]) -> bool {
    let (point_x, point_y) = point;
    if boundary.len() < 3 {
        return false;
    }

    let mut is_inside = false;
    for ((vert_x, vert_y), (latest_x, latest_y)) in boundary_edges(boundary) {
        if ((vert_y > point_y) != (latest_y > point_y))
            && (point_x < (latest_x - vert_x) * (point_y - vert_y) / (latest_y - vert_y) + vert_x)
        {
            is_inside = !is_inside;
        }
    }

    is_inside
}

/// The visibility graph's full node set: the polygon's own vertices, plus
/// `start` and `goal` appended at the end. Everything downstream (edges,
/// Dijkstra) works by index into this combined list, so the indices where
/// `start`/`goal` landed are returned alongside it.
fn visibility_nodes(
    boundary: &[(f32, f32)],
    start: (f32, f32),
    goal: (f32, f32),
) -> (Vec<(f32, f32)>, usize, usize) {
    let mut nodes = boundary.to_vec();
    let start_index = nodes.len();
    nodes.push(start);
    let goal_index = nodes.len();
    nodes.push(goal);
    (nodes, start_index, goal_index)
}

/// Read this crate's `gui.walkable` convention out of a room's `extra` data
/// (see [`crate::hotspot::hotspot_rect`]/[`crate::asset::sprite_key`] for
/// the same per-key convention at the object level). Returns `None` for
/// anything not yet authored or malformed — never panics on incomplete
/// content.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn walkable_area(extra: &HashMap<String, ExtraValue>) -> Option<WalkableArea> {
    let gui = as_table(extra.get("gui")?)?;
    let boundary = as_array(gui.get("walkable")?)?
        .iter()
        .map(|vertex| {
            let pair = as_array(vertex)?;
            Some((as_number(pair.first()?)?, as_number(pair.get(1)?)?))
        })
        .collect::<Option<Vec<(f32, f32)>>>()?;

    Some(WalkableArea { boundary })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn table(pairs: Vec<(&str, ExtraValue)>) -> ExtraValue {
        ExtraValue::Table(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn point(x: f64, y: f64) -> ExtraValue {
        ExtraValue::Array(vec![ExtraValue::Float(x), ExtraValue::Float(y)])
    }

    /// A square room: any two interior points see each other directly.
    fn square_room() -> WalkableArea {
        WalkableArea {
            boundary: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        }
    }

    /// An L-shaped room (a 10x10 square with the top-right 6x6 quadrant
    /// removed): the reflex vertex at (4.0, 4.0) forces a bend when walking
    /// between the two arms of the L.
    fn l_shaped_room() -> WalkableArea {
        WalkableArea {
            boundary: vec![
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 4.0),
                (4.0, 4.0),
                (4.0, 10.0),
                (0.0, 10.0),
            ],
        }
    }

    #[test]
    fn orientation_is_positive_for_a_counterclockwise_turn() {
        assert!(orientation((0.0, 0.0), (1.0, 0.0), (1.0, 1.0)) > 0.0);
    }

    #[test]
    fn orientation_is_negative_for_a_clockwise_turn() {
        assert!(orientation((0.0, 0.0), (1.0, 0.0), (1.0, -1.0)) < 0.0);
    }

    #[test]
    fn orientation_is_zero_for_collinear_points() {
        assert_eq!(orientation((0.0, 0.0), (1.0, 0.0), (2.0, 0.0)), 0.0);
    }

    #[test]
    fn segments_intersect_detects_a_proper_crossing() {
        assert!(segments_intersect(
            (0.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
            (2.0, 0.0)
        ));
    }

    #[test]
    fn segments_intersect_ignores_a_shared_endpoint() {
        // Touching at a single shared point is not a proper crossing — see
        // has_line_of_sight, which relies on this to avoid a candidate
        // segment falsely "blocking itself" against its own vertex.
        assert!(!segments_intersect(
            (0.0, 0.0),
            (1.0, 1.0),
            (1.0, 1.0),
            (2.0, 0.0)
        ));
    }

    #[test]
    fn segments_intersect_is_false_for_non_crossing_segments() {
        assert!(!segments_intersect(
            (0.0, 0.0),
            (1.0, 0.0),
            (0.0, 1.0),
            (1.0, 1.0)
        ));
    }

    #[test]
    fn point_in_polygon_is_true_for_an_interior_point() {
        assert!(point_in_polygon((5.0, 5.0), &square_room().boundary));
    }

    #[test]
    fn point_in_polygon_is_false_for_an_exterior_point() {
        assert!(!point_in_polygon((50.0, 50.0), &square_room().boundary));
    }

    #[test]
    fn point_in_polygon_is_false_inside_an_l_shapes_removed_quadrant() {
        assert!(!point_in_polygon((8.0, 8.0), &l_shaped_room().boundary));
    }

    #[test]
    fn has_line_of_sight_is_true_between_adjacent_boundary_vertices() {
        let boundary = &square_room().boundary;
        assert!(has_line_of_sight((0.0, 0.0), (10.0, 0.0), boundary));
    }

    #[test]
    fn has_line_of_sight_is_false_when_blocked_by_a_concave_notch() {
        let boundary = &l_shaped_room().boundary;
        assert!(!has_line_of_sight((8.0, 1.0), (1.0, 8.0), boundary));
    }

    #[test]
    fn direct_line_of_sight_yields_a_single_waypoint_path() {
        let area = square_room();
        let path = find_path(&area, (1.0, 1.0), (9.0, 9.0));
        assert_eq!(path, Some(vec![(9.0, 9.0)]));
    }

    #[test]
    fn path_bends_around_a_reflex_corner_in_an_l_shaped_room() {
        let area = l_shaped_room();
        // A straight line from the bottom arm to the left arm would cut
        // through the removed top-right quadrant, so the path must route
        // through the reflex vertex at (4.0, 4.0).
        let path = find_path(&area, (8.0, 1.0), (1.0, 8.0));
        assert_eq!(path, Some(vec![(4.0, 4.0), (1.0, 8.0)]));
    }

    #[test]
    fn returns_none_when_start_is_outside_the_walkable_area() {
        let area = square_room();
        assert_eq!(find_path(&area, (-5.0, -5.0), (5.0, 5.0)), None);
    }

    #[test]
    fn returns_none_when_goal_is_outside_the_walkable_area() {
        let area = square_room();
        assert_eq!(find_path(&area, (5.0, 5.0), (50.0, 50.0)), None);
    }

    #[test]
    fn walkable_area_reads_the_gui_walkable_convention() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            table(vec![(
                "walkable",
                ExtraValue::Array(vec![
                    point(0.0, 0.0),
                    point(10.0, 0.0),
                    point(10.0, 10.0),
                    point(0.0, 10.0),
                ]),
            )]),
        );
        assert_eq!(
            walkable_area(&extra),
            Some(WalkableArea {
                boundary: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            })
        );
    }

    #[test]
    fn walkable_area_is_none_when_not_authored() {
        assert_eq!(walkable_area(&HashMap::new()), None);
    }

    #[test]
    fn walkable_area_is_none_when_malformed() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            table(vec![(
                "walkable",
                ExtraValue::Str("not a polygon".to_string()),
            )]),
        );
        assert_eq!(walkable_area(&extra), None);
    }
}
