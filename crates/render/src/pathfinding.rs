//! Click-to-walk pathfinding over a room's authored floor layout, the
//! standard adventure-game approach (Wintermute, Visionaire, AGS's modern
//! pathfinder): a room's walkable area is a polygon (an outer boundary,
//! plus zero or more interior obstacle holes - furniture, pillars, ... -
//! routed around rather than walked through), and a path between two
//! points is the shortest route through the visibility graph of that
//! polygon's vertices (any two vertices - plus the start/end points - with
//! an unobstructed line of sight between them are connected, then shortest
//! path is Dijkstra/A* over that graph).
//!
//! Like [`crate::hotspot`]/[`crate::asset`], a room's walkable area is
//! authored data (`extra.gui.walkable`, read by [`walkable_area`] as a
//! table with a required `boundary` - a flat list of `[x, y]` points in
//! room-local coordinates, same space as `gui.hotspot` - and an optional
//! `holes`, a list of such point lists, defaulting to none) - `render`
//! supplies the algorithm, a specific game supplies the room geometry.

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::collections::HashSet;

use typegin_core::ExtraValue;

use crate::extra::as_polygon;
use crate::extra::{as_array, as_table};

/// A room's walkable floor: an outer polygon boundary (room-local
/// coordinates, vertices in order), plus zero or more interior obstacle
/// holes - each also a polygon, in the same coordinate space - that a path
/// must route around rather than cross.
#[derive(Debug, Clone, PartialEq)]
pub struct WalkableArea {
    pub boundary: Vec<(f32, f32)>,
    pub holes: Vec<Vec<(f32, f32)>>,
}

/// Find the shortest walkable path from `start` to `goal` inside `area`, as
/// a sequence of waypoints to walk through in order (not including `start`
/// itself, since a caller already knows where it's standing; `goal` is
/// always the last waypoint when a path exists). A direct, unobstructed
/// line of sight yields a single-waypoint path (just `goal`); a concave
/// boundary may force the path through one or more of the polygon's own
/// vertices to stay inside the walkable area.
///
/// Returns `None` if `start` itself lies outside `area` - that's a
/// precondition violation (the caller's own position should always be
/// valid), not a normal click. `goal` is different: a click can easily land
/// outside the walkable area (past the boundary, or on an obstacle inside a
/// hole) - rather than refusing to move at all, an unwalkable `goal` is
/// clamped to the nearest walkable point via [`nearest_walkable_point`], so
/// the character walks "as far as possible" toward where the player clicked.
#[must_use]
pub fn find_path(
    area: &WalkableArea,
    start: (f32, f32),
    goal: (f32, f32),
) -> Option<Vec<(f32, f32)>> {
    if !is_walkable(start, area) {
        return None;
    }
    let goal = if is_walkable(goal, area) {
        goal
    } else {
        nearest_walkable_point(goal, area)
    };

    let (nodes, start_index, goal_index) = visibility_nodes(area, start, goal);
    let graph = build_visibility_graph(&nodes, area);

    dijkstra_evaluation(&graph, &nodes, start_index, goal_index)
}

/// Whether `point` is actually walkable: inside `area`'s outer boundary,
/// and not inside (or on the boundary of) any of its holes - a point can't
/// be a valid `start`/`goal` sitting on top of an obstacle.
fn is_walkable(point: (f32, f32), area: &WalkableArea) -> bool {
    let is_in_polygon = point_in_polygon(point, &area.boundary);
    let is_in_hole = area.holes.iter().any(|hole| point_in_polygon(point, hole));
    is_in_polygon && !is_in_hole
}

/// The closest point to `point` that's actually walkable in `area` - used
/// by [`find_path`] to let a click that lands outside the walkable area (or
/// inside a hole) still walk "as far as possible" toward it, rather than
/// refusing to move at all.
///
/// Only ever called on a `point` that's already confirmed *not*
/// [`is_walkable`], so `point` is either outside `area`'s boundary, or
/// inside exactly one hole - never both, and never fully walkable already.
fn nearest_walkable_point(point: (f32, f32), area: &WalkableArea) -> (f32, f32) {
    if !point_in_polygon(point, &area.boundary) {
        let nearest = nearest_point_on_polygon(point, &area.boundary);
        // Nudge just clear of the boundary line itself, not merely onto
        // it: `point_in_polygon`'s crossing-number test classifies an
        // exactly-on-the-edge point as inside on some edges and outside on
        // others (a documented, deliberate asymmetry - see its own doc
        // comment), so landing a walker exactly on the line can leave it
        // standing somewhere `is_walkable` itself disagrees is walkable,
        // permanently refusing every future move from there.
        return move_toward(nearest, centroid(&area.boundary), CLEARANCE);
    }
    let containing_hole = area.holes.iter().find(|hole| point_in_polygon(point, hole));
    match containing_hole {
        // Same reasoning as above, mirrored: nudge away from the hole's
        // own centroid instead of toward it, to clear its edge outward
        // rather than the boundary's edge inward.
        Some(hole) => {
            let nearest = nearest_point_on_polygon(point, hole);
            move_toward(nearest, centroid(hole), -CLEARANCE)
        }
        // Defensive only: per this function's contract, `point` is always
        // inside `area.boundary` and inside some hole by this point.
        None => point,
    }
}

/// How far [`nearest_walkable_point`] nudges a clamped point clear of the
/// boundary/hole edge line it landed on. Small enough to be visually
/// imperceptible at this crate's typical room scale, large enough to
/// reliably clear `point_in_polygon`'s exact-on-the-line ambiguity.
const CLEARANCE: f32 = 0.5;

/// The average of `polygon`'s vertices - a rough "which way is inward"
/// reference for [`nearest_walkable_point`]'s nudge, not a precise
/// geometric centroid (which, for a strongly concave polygon, might not
/// even land inside it). Harmless here since it only steers a tiny nudge
/// direction, never the clamped point's actual position.
#[allow(clippy::cast_precision_loss)]
fn centroid(polygon: &[(f32, f32)]) -> (f32, f32) {
    let count = polygon.len() as f32;
    let sum = polygon.iter().fold((0.0, 0.0), |acc, vertex| {
        (acc.0 + vertex.0, acc.1 + vertex.1)
    });
    (sum.0 / count, sum.1 / count)
}

/// Move `point` a small `amount` toward `target` (or away from it, for a
/// negative `amount`).
#[allow(clippy::float_cmp)]
fn move_toward(point: (f32, f32), target: (f32, f32), amount: f32) -> (f32, f32) {
    let gap = distance(point, target);
    if gap == 0.0 {
        return point;
    }
    let direction = ((target.0 - point.0) / gap, (target.1 - point.1) / gap);
    (
        point.0 + direction.0 * amount,
        point.1 + direction.1 * amount,
    )
}

/// The closest point on any edge of `polygon` to `point` - the minimum
/// over every edge's [`nearest_point_on_segment`]. Used by
/// [`nearest_walkable_point`] against either the outer boundary or a
/// specific hole; either way it's "the nearest point on this polygon's own
/// edges," so the two cases share this one helper.
fn nearest_point_on_polygon(point: (f32, f32), polygon: &[(f32, f32)]) -> (f32, f32) {
    boundary_edges(polygon)
        .map(|(segment_start, segment_end)| {
            nearest_point_on_segment(point, segment_start, segment_end)
        })
        .min_by(|first_candidate, second_candidate| {
            distance(point, *first_candidate).total_cmp(&distance(point, *second_candidate))
        })
        .expect("polygon has at least one edge")
}

/// The closest point to `point` that lies on the line segment
/// `segment_start`-`segment_end`: the perpendicular projection of `point`
/// onto the (infinite) line through them, clamped to stay between the two
/// endpoints when that projection would otherwise fall outside the segment.
#[allow(clippy::float_cmp)]
fn nearest_point_on_segment(
    point: (f32, f32),
    segment_start: (f32, f32),
    segment_end: (f32, f32),
) -> (f32, f32) {
    let segment = (
        segment_end.0 - segment_start.0,
        segment_end.1 - segment_start.1,
    );
    let segment_len_sq = segment.0 * segment.0 + segment.1 * segment.1;
    if segment_len_sq == 0.0 {
        // Degenerate segment (segment_start == segment_end exactly, so
        // there's nothing to project onto) - the "segment" is just the
        // single point `segment_start`.
        return segment_start;
    }

    let to_point = (point.0 - segment_start.0, point.1 - segment_start.1);
    let t = ((to_point.0 * segment.0 + to_point.1 * segment.1) / segment_len_sq).clamp(0.0, 1.0);
    (
        segment_start.0 + t * segment.0,
        segment_start.1 + t * segment.1,
    )
}

/// One entry in the priority queue: a candidate distance to reach `node`.
/// `BinaryHeap` is a max-heap, so [`Ord`] is implemented *reversed*
/// (`other` compared against `self`) to make the *smallest* distance pop
/// first - the standard trick for using it as a min-heap.
#[derive(Debug, Clone, Copy)]
struct HeapEntry {
    distance: f32,
    node: usize,
}

// `PartialEq`/`Eq` are implemented manually (distance-only) to stay
// consistent with the `Ord`/`PartialOrd` impls below - a derived
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
        // `0.0`, so it's always finite - `partial_cmp` never sees a `NaN`.
        other.distance.partial_cmp(&self.distance).unwrap()
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Run Dijkstra over `graph` from `start` to `goal` (both node indices into
/// `nodes`), and reconstruct the shortest path as actual points - or `None`
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

    // Walk `predecessors` backward from `goal` to `start`, then reverse -
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

/// Straight-line (Euclidean) distance between two points - the visibility
/// graph's edge weight, since a path's real cost is the physical distance
/// walked, not the number of hops between vertices.
fn distance(point_a: (f32, f32), point_b: (f32, f32)) -> f32 {
    (point_a.0 - point_b.0).hypot(point_a.1 - point_b.1)
}

/// The visibility graph as an adjacency list: each node's index maps to the
/// indices of every other node it has an unobstructed line of sight to,
/// tested against `area` - the room `nodes` was built from (see
/// `visibility_nodes`). Takes `area` as its own parameter, separate from
/// `nodes`, since `has_line_of_sight` needs to test candidate segments
/// against the room's actual boundary/hole edges, not against every other
/// graph node (which include `start`/`goal`, not real polygon edges).
///
/// An adjacency list (rather than a flat list of edges) is what lets
/// Dijkstra look up "this node's neighbors" in O(1) instead of scanning
/// every edge, and keeps the whole graph working by index - matching
/// `visited`/`start_index`/`goal_index` in `dijkstra_evaluation` - instead
/// of needing to hash or compare `(f32, f32)` points directly.
///
/// `i + 1..nodes.len()` (rather than `0..nodes.len()`) considers each
/// unordered pair exactly once - visibility is symmetric, so testing both
/// `(i, j)` and `(j, i)` would just repeat the same check. Since it's
/// symmetric, though, a discovered edge is inserted into *both* directions'
/// neighbor lists - otherwise Dijkstra could only ever walk it one way.
fn build_visibility_graph(nodes: &[(f32, f32)], area: &WalkableArea) -> HashMap<usize, Vec<usize>> {
    let mut graph: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            if has_line_of_sight(nodes[i], nodes[j], area) {
                graph.entry(i).or_default().push(j);
                graph.entry(j).or_default().push(i);
            }
        }
    }
    graph
}

/// Whether the segment `a1-a2` has an unobstructed line of sight inside
/// `area` - i.e. it stays inside the outer boundary, and outside every
/// hole, rather than cutting across the exterior or through an obstacle.
///
/// This is the practical visibility test (midpoint-inside plus no proper
/// edge crossing), not the fully rigorous local-vertex-angle visibility
/// test from computational geometry - a deliberate choice for a typical
/// room floor plan, but one that could in principle misjudge visibility on
/// extremely thin polygon slivers or near-tangent segments.
fn has_line_of_sight(a1: (f32, f32), a2: (f32, f32), area: &WalkableArea) -> bool {
    // A segment that never crosses the boundary or a hole is either
    // entirely inside the walkable area or entirely outside/inside an
    // obstacle; checking one interior sample point - the midpoint - is
    // enough to tell which.
    let midpoint = (f32::midpoint(a1.0, a2.0), f32::midpoint(a1.1, a2.1));
    if !is_walkable(midpoint, area) {
        return false;
    }

    boundary_edges(&area.boundary)
        .chain(area.holes.iter().flat_map(|hole| boundary_edges(hole)))
        .all(|(b1, b2)| !segments_intersect(a1, a2, b1, b2))
}

/// Every edge of `boundary` as a `(vertex, previous vertex)` pair, including
/// the wraparound edge connecting the last vertex back to the first. Shared
/// by [`point_in_polygon`] and [`has_line_of_sight`], which both need to
/// walk a polygon's edges the same way - this indexing (`i` paired with
/// `i`'s predecessor, wrapping via `% len`) is exactly the kind of thing
/// that's easy to get subtly wrong twice.
fn boundary_edges(boundary: &[(f32, f32)]) -> impl Iterator<Item = ((f32, f32), (f32, f32))> + '_ {
    let len = boundary.len();
    (0..len).map(move |i| (boundary[i], boundary[(i + len - 1) % len]))
}

/// Sign of the cross product `(line_end - line_start) x (point - line_start)`:
/// positive when `point` is a counterclockwise turn from
/// `line_start -> line_end`, negative when clockwise, zero when the three
/// are collinear (this includes `point` being one of `line_start`/`line_end`
/// themselves, since a point has zero cross product with itself).
fn orientation(line_start: (f32, f32), line_end: (f32, f32), point: (f32, f32)) -> f32 {
    (line_end.0 - line_start.0) * (point.1 - line_start.1)
        - (line_end.1 - line_start.1) * (point.0 - line_start.0)
}

/// Whether segment `a1-a2` properly crosses segment `b1-b2` - i.e. each
/// segment's endpoints fall on strictly opposite sides of the other
/// segment's line. Requiring *strict*, nonzero opposite signs (rather than
/// just "different") is deliberate: it's what keeps two segments that only
/// touch at a shared endpoint (collinear, orientation zero) from counting
/// as crossing - exactly the case a visibility-graph candidate segment hits
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

/// The [pnpoly](https://wrfranklin.org/Research/Short_Notes/pnpoly.html)
/// algorithm to determine if a point is inside the given polygon.
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

/// The visibility graph's full node set: `area`'s outer boundary vertices,
/// every hole's vertices, plus `start` and `goal` appended at the end -
/// routing around a hole means being able to bend at its corners, the same
/// way routing around a concave boundary notch bends at *its* corners, so a
/// hole's vertices need to be graph nodes too, not just the boundary's.
/// Everything downstream (edges, Dijkstra) works by index into this
/// combined list, so the indices where `start`/`goal` landed are returned
/// alongside it.
fn visibility_nodes(
    area: &WalkableArea,
    start: (f32, f32),
    goal: (f32, f32),
) -> (Vec<(f32, f32)>, usize, usize) {
    let mut nodes = area.boundary.clone();
    for hole in &area.holes {
        nodes.extend(hole.iter().copied());
    }
    let start_index = nodes.len();
    nodes.push(start);
    let goal_index = nodes.len();
    nodes.push(goal);
    (nodes, start_index, goal_index)
}

/// Read this crate's `gui.walkable` convention out of a room's `extra`
/// data: a table with a required `boundary` (a flat list of `[x, y]`
/// points, parsed by `as_polygon`) and an optional `holes` (a list of
/// such point lists, one per obstacle - defaulting to no holes when the key
/// is absent). Returns `None` for anything not yet authored or malformed
/// (see [`crate::hotspot::hotspot_rect`]/[`crate::asset::sprite_key`] for
/// the same per-key convention at the object level) - never panics on
/// incomplete content.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn walkable_area(extra: &HashMap<String, ExtraValue>) -> Option<WalkableArea> {
    let gui = as_table(extra.get("gui")?)?;
    let walkable = as_table(gui.get("walkable")?)?;

    let boundary = as_polygon(walkable.get("boundary")?)?;
    let holes = match walkable.get("holes") {
        Some(holes) => as_array(holes)?
            .iter()
            .map(as_polygon)
            .collect::<Option<Vec<_>>>()?,
        None => Vec::new(),
    };

    Some(WalkableArea { boundary, holes })
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
            holes: Vec::new(),
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
            holes: Vec::new(),
        }
    }

    /// A square room (same outer boundary as `square_room`) with a single
    /// 2x2 obstacle hole centered in the middle of the floor - a table or
    /// pillar a path must route around rather than cross.
    fn square_with_center_hole_room() -> WalkableArea {
        WalkableArea {
            boundary: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            holes: vec![vec![(4.0, 4.0), (6.0, 4.0), (6.0, 6.0), (4.0, 6.0)]],
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
        // Touching at a single shared point is not a proper crossing - see
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
        assert!(has_line_of_sight((0.0, 0.0), (10.0, 0.0), &square_room()));
    }

    #[test]
    fn has_line_of_sight_is_false_when_blocked_by_a_concave_notch() {
        assert!(!has_line_of_sight((8.0, 1.0), (1.0, 8.0), &l_shaped_room()));
    }

    #[test]
    fn has_line_of_sight_is_false_when_blocked_by_a_hole() {
        // Straight through the hole's dead center - unambiguous, unlike a
        // segment that merely grazes a hole's edge or corner.
        let area = square_with_center_hole_room();
        assert!(!has_line_of_sight((1.0, 5.0), (9.0, 5.0), &area));
    }

    #[test]
    fn has_line_of_sight_is_true_between_a_point_and_an_unobstructed_hole_corner() {
        // Approaches the hole's corner (4.0, 6.0) from outside, without
        // ever crossing into the hole's interior - touching a hole's own
        // vertex is exactly how a path bends around it (see
        // segments_intersect_ignores_a_shared_endpoint).
        let area = square_with_center_hole_room();
        assert!(has_line_of_sight((1.0, 2.0), (4.0, 6.0), &area));
    }

    #[test]
    fn nearest_point_on_segment_projects_perpendicularly_when_within_the_segment() {
        let nearest = nearest_point_on_segment((5.0, 2.0), (0.0, 0.0), (10.0, 0.0));
        assert_eq!(nearest, (5.0, 0.0));
    }

    #[test]
    fn nearest_point_on_segment_clamps_to_an_endpoint_when_the_projection_falls_outside() {
        let nearest = nearest_point_on_segment((15.0, 2.0), (0.0, 0.0), (10.0, 0.0));
        assert_eq!(nearest, (10.0, 0.0));
    }

    #[test]
    fn nearest_walkable_point_projects_onto_the_boundary_when_point_is_outside_it() {
        // The raw projection onto the top edge would be (5.0, 10.0), but
        // the result is nudged CLEARANCE (0.5) further in, toward the
        // boundary's centroid, to clear the edge itself (see
        // nearest_walkable_point's doc comment).
        let nearest = nearest_walkable_point((5.0, 50.0), &square_room());
        assert_eq!(nearest, (5.0, 9.5));
    }

    #[test]
    fn nearest_walkable_point_projects_onto_the_containing_holes_edge() {
        // The raw projection onto the hole's right edge would be
        // (6.0, 5.0), but the result is nudged CLEARANCE (0.5) further
        // out, away from the hole's centroid, to clear that edge.
        let nearest = nearest_walkable_point((5.2, 5.0), &square_with_center_hole_room());
        assert_eq!(nearest, (6.5, 5.0));
    }

    #[test]
    fn nearest_walkable_point_result_is_itself_walkable() {
        // Regression test: a clamped point that lands exactly on a
        // boundary/hole edge line can be mis-classified by
        // point_in_polygon's crossing-number test (see
        // nearest_walkable_point's doc comment) - if that ever happens,
        // the avatar gets stranded on a point it can never walk away from
        // again, since find_path's own is_walkable(start, ...) guard would
        // reject every future move.
        let hole_area = square_with_center_hole_room();
        assert!(is_walkable(
            nearest_walkable_point((5.2, 5.0), &hole_area),
            &hole_area
        ));

        let boundary_area = square_room();
        assert!(is_walkable(
            nearest_walkable_point((5.0, 50.0), &boundary_area),
            &boundary_area
        ));
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
    fn path_routes_around_a_hole() {
        let area = square_with_center_hole_room();
        // The direct line from (1.0, 2.0) to (9.0, 8.5) cuts straight
        // through the hole; routing via its (4.0, 6.0) corner is strictly
        // shorter than via its (6.0, 4.0) corner (this asymmetry is
        // deliberate - a symmetric start/goal would tie the two routes and
        // make the expected path implementation-dependent).
        let path = find_path(&area, (1.0, 2.0), (9.0, 8.5));
        assert_eq!(path, Some(vec![(4.0, 6.0), (9.0, 8.5)]));
    }

    #[test]
    fn direct_line_of_sight_is_unaffected_by_a_hole_elsewhere() {
        let area = WalkableArea {
            boundary: square_room().boundary,
            holes: vec![vec![(1.0, 1.0), (2.0, 1.0), (2.0, 2.0), (1.0, 2.0)]],
        };
        // Nowhere near the hole tucked in the corner - a hole must not
        // spuriously block paths elsewhere in the room.
        let path = find_path(&area, (5.0, 5.0), (9.0, 9.0));
        assert_eq!(path, Some(vec![(9.0, 9.0)]));
    }

    #[test]
    fn returns_none_when_start_is_inside_a_hole() {
        // start is a precondition, not a click - unlike goal, it's never
        // clamped (see find_path's doc comment).
        let area = square_with_center_hole_room();
        assert_eq!(find_path(&area, (5.0, 5.0), (1.0, 1.0)), None);
    }

    #[test]
    fn returns_none_when_start_is_outside_the_walkable_area() {
        let area = square_room();
        assert_eq!(find_path(&area, (-5.0, -5.0), (5.0, 5.0)), None);
    }

    #[test]
    fn find_path_clamps_a_goal_outside_the_boundary_to_the_nearest_edge_point() {
        let area = square_room();
        // (5.0, 50.0) is far above the room; the nearest walkable point is
        // just inside the top edge, (5.0, 9.5) (see
        // nearest_walkable_point_projects_onto_the_boundary_... for the
        // 0.5 clearance nudge) - directly visible from the center, so a
        // single-waypoint path.
        let path = find_path(&area, (5.0, 5.0), (5.0, 50.0));
        assert_eq!(path, Some(vec![(5.0, 9.5)]));
    }

    #[test]
    fn find_path_clamps_a_goal_inside_a_hole_to_the_holes_nearest_edge_point() {
        let area = square_with_center_hole_room();
        // (5.2, 5.0) is inside the hole, closer to its right edge than any
        // other edge; clamped (with its 0.5 clearance nudge, see
        // nearest_walkable_point_projects_onto_the_containing_holes_edge)
        // to (6.5, 5.0), directly visible from the right of the hole - no
        // bend needed - keeping this test focused on clamping alone, not
        // also on path-bending (see path_routes_around_a_hole for that).
        let path = find_path(&area, (8.0, 5.0), (5.2, 5.0));
        assert_eq!(path, Some(vec![(6.5, 5.0)]));
    }

    fn square_boundary_points() -> Vec<ExtraValue> {
        vec![
            point(0.0, 0.0),
            point(10.0, 0.0),
            point(10.0, 10.0),
            point(0.0, 10.0),
        ]
    }

    #[test]
    fn walkable_area_reads_holes_from_the_gui_walkable_convention() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            table(vec![(
                "walkable",
                table(vec![
                    ("boundary", ExtraValue::Array(square_boundary_points())),
                    (
                        "holes",
                        ExtraValue::Array(vec![ExtraValue::Array(vec![
                            point(4.0, 4.0),
                            point(6.0, 4.0),
                            point(6.0, 6.0),
                            point(4.0, 6.0),
                        ])]),
                    ),
                ]),
            )]),
        );
        assert_eq!(
            walkable_area(&extra),
            Some(WalkableArea {
                boundary: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
                holes: vec![vec![(4.0, 4.0), (6.0, 4.0), (6.0, 6.0), (4.0, 6.0)]],
            })
        );
    }

    #[test]
    fn walkable_area_defaults_to_no_holes_when_holes_key_is_omitted() {
        let mut extra = HashMap::new();
        extra.insert(
            "gui".to_string(),
            table(vec![(
                "walkable",
                table(vec![(
                    "boundary",
                    ExtraValue::Array(square_boundary_points()),
                )]),
            )]),
        );
        assert_eq!(
            walkable_area(&extra),
            Some(WalkableArea {
                boundary: vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
                holes: Vec::new(),
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

    #[test]
    fn walkable_area_is_none_when_boundary_key_is_missing() {
        let mut extra = HashMap::new();
        extra.insert("gui".to_string(), table(vec![("walkable", table(vec![]))]));
        assert_eq!(walkable_area(&extra), None);
    }
}
