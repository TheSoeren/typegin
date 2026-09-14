//! Demonstration that `crates/render` (a reusable rendering engine, no game
//! content of its own - see AGENTS.md) can drive the *same* shared example
//! content `examples/text` and `crates/text-proto` use
//! (`examples/data/`), through a real macroquad window: boot a
//! `GameEngine`, draw the current room's hotspots, show a radial verb coin
//! on hover, walk a placeholder avatar to the clicked point via
//! `render::pathfinding`, then execute the highest-priority verb on
//! arrival.
//!
//! All the actual logic (hit-testing, coin layout, animation, pathfinding)
//! lives in `render`'s library, not here - this file only wires it
//! together and draws. A real game would replace `candidate_object_ids`
//! below with a proper "list this room's objects" query once core exposes
//! one (see the comment on it) and would own its own asset lookup instead
//! of drawing plain rectangles.

use std::collections::{HashMap, VecDeque};

use macroquad::prelude::*;
use typegin_core::{Action, GameEngine, Locator, ObjectId, ObjectResolution, Target, WorldData};

use render::asset::{self, AssetProvider};
use render::hotspot::{self, Rect};
use render::pathfinding;
use render::{coin, tween::Tween};

/// Avatar walk speed, in pixels per second - this example's own concern
/// (`render`'s `Tween` has no notion of speed, only "reach `target` in
/// `duration` seconds"; a specific game picks its own pacing).
const WALK_SPEED: f32 = 220.0;

const DATA_FILES: [&str; 6] = [
    "globals.yaml",
    "items.yaml",
    "rooms.yaml",
    "interactions.yaml",
    "npcs.yaml",
    "triggers.yaml",
];

fn load_shared_world() -> WorldData {
    let data_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../data");
    let mut yaml = String::new();
    for file_name in DATA_FILES {
        yaml.push_str(&std::fs::read_to_string(format!("{data_dir}/{file_name}")).unwrap());
        yaml.push('\n');
    }
    WorldData::from_yaml(&yaml).expect("shared example data parses")
}

/// The example's fixed object list, since core doesn't yet expose a "list
/// this room's objects with their full `ObjectInfo`" query - only names
/// (`WorldState::room_object_names`) or a lookup by an id you already have.
/// A real game built on `render` needs that query added to core; this
/// example just hardcodes the ids the shared starting room happens to have,
/// to prove the rest of the pipeline. `door-grate` is listed before
/// `cell-door` deliberately: their authored hotspots overlap, and the
/// first match in this list wins, so the more specific one should come
/// first.
fn candidate_object_ids() -> Vec<ObjectId> {
    ["chair", "table", "padding", "door-grate", "cell-door"]
        .into_iter()
        .map(ObjectId::new)
        .collect()
}

struct SceneHotspot {
    target: Target,
    rect: Rect,
    label: String,
    sprite: Option<Texture2D>,
}

/// Load this example's placeholder art (see `examples/pnc/assets/`) into an
/// `AssetProvider` keyed by the `extra.gui.sprite` values authored in
/// `examples/data/items.yaml` - proves `render::asset`'s wiring end to end,
/// even though these are flat-color placeholders, not real art (see
/// AGENTS.md: this crate stays throwaway).
async fn load_assets() -> HashMap<String, Texture2D> {
    let asset_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
    let sprite_keys = [
        "chair_default",
        "table_default",
        "padding_default",
        "door_grate_default",
        "cell_door_default",
    ];
    let mut assets = HashMap::new();
    for key in sprite_keys {
        let path = format!("{asset_dir}/{key}.png");
        let texture = load_texture(&path)
            .await
            .unwrap_or_else(|err| panic!("failed to load placeholder asset {path}: {err}"));
        assets.insert(key.to_string(), texture);
    }
    assets
}

/// Every candidate object that's both currently in the room and has
/// authored `gui.hotspot` data. Anything currently in the room *without*
/// hotspot data is returned separately, so it stays visible (as a text
/// fallback) instead of silently vanishing.
fn visible_hotspots(
    engine: &GameEngine,
    assets: &HashMap<String, Texture2D>,
) -> (Vec<SceneHotspot>, Vec<String>) {
    let world = engine.world();
    let mut placed = Vec::new();
    let mut unplaced = Vec::new();
    for id in candidate_object_ids() {
        if !matches!(world.get_object_from_room(&id), ObjectResolution::Found(_)) {
            continue;
        }
        let Some(info) = world.object_info(&id) else {
            continue;
        };
        match hotspot::hotspot_rect(&info.extra) {
            Some(rect) => {
                let sprite = asset::sprite_key(&info.extra).and_then(|key| assets.asset(key));
                placed.push(SceneHotspot {
                    target: Target::Object(id),
                    rect,
                    label: info.name,
                    sprite,
                });
            }
            None => unplaced.push(info.name),
        }
    }
    (placed, unplaced)
}

fn room_description(engine: &GameEngine) -> String {
    match engine
        .world()
        .current_room_extra()
        .get("description")
        .map(typegin_core::ExtraValue::to_owned)
    {
        Some(typegin_core::ExtraValue::Str(desc)) => desc,
        _ => "(no description authored)".to_string(),
    }
}

#[macroquad::main("pnc-example")]
async fn main() {
    let world_data = load_shared_world();
    let mut engine = GameEngine::get(&world_data);
    let assets = load_assets().await;
    let mut coin_alpha = Tween::settled(0.0_f32);

    // A placeholder player avatar (see AGENTS.md: this crate stays
    // throwaway, plain shapes not real art) that walks - leg by leg - along
    // a `render::pathfinding::find_path` route before a clicked verb
    // actually executes, rather than executing instantly on click.
    let mut avatar_position = Tween::settled((400.0_f32, 550.0_f32));
    let mut remaining_path: VecDeque<(f32, f32)> = VecDeque::new();
    let mut pending_action: Option<Action> = None;

    loop {
        clear_background(Color::from_rgba(20, 20, 24, 255));

        let (hotspots, unplaced) = visible_hotspots(&engine, &assets);
        let mouse = mouse_position();
        let hovered = hotspots.iter().position(|h| h.rect.contains(mouse));
        let walkable = pathfinding::walkable_area(&engine.world().current_room_extra());

        coin_alpha.set_target(if hovered.is_some() { 1.0 } else { 0.0 }, 0.15);
        coin_alpha.advance(get_frame_time());
        advance_avatar(
            get_frame_time(),
            &mut avatar_position,
            &mut remaining_path,
            &mut pending_action,
            &mut engine,
        );

        draw_text(&room_description(&engine), 20.0, 30.0, 20.0, WHITE);
        draw_hotspots(&hotspots, hovered);

        if !unplaced.is_empty() {
            draw_text(
                &format!("(no hotspot authored yet: {})", unplaced.join(", ")),
                20.0,
                screen_height() - 20.0,
                16.0,
                GRAY,
            );
        }

        draw_walkable_and_avatar(walkable.as_ref(), avatar_position.current());

        let clicked_verb = draw_verb_coin_and_detect_click(
            &hotspots,
            hovered,
            &engine,
            coin_alpha.current(),
            pending_action.is_none(),
        );

        if let Some(verb) = clicked_verb {
            let target = hotspots[hovered.expect("clicked_verb only set while hovered")]
                .target
                .clone();
            let action = action_for(verb, target);
            begin_walk_or_execute(
                Some(action),
                walkable.as_ref(),
                avatar_position.current(),
                mouse,
                &mut remaining_path,
                &mut pending_action,
                &mut engine,
            );
        } else if hovered.is_none()
            && is_mouse_button_pressed(MouseButton::Left)
            && pending_action.is_none()
        {
            // Clicking empty floor (no hotspot hovered, so no verb coin to
            // choose from) just walks the avatar there - same mechanism,
            // but with nothing to execute on arrival.
            begin_walk_or_execute(
                None,
                walkable.as_ref(),
                avatar_position.current(),
                mouse,
                &mut remaining_path,
                &mut pending_action,
                &mut engine,
            );
        }

        next_frame().await;
    }
}

/// Advance the avatar's walk-in-progress by `dt` seconds: once the current
/// leg settles, either start the next waypoint's leg or - once the whole
/// path is walked - execute whatever verb triggered the walk.
fn advance_avatar(
    dt: f32,
    avatar_position: &mut Tween<(f32, f32)>,
    remaining_path: &mut VecDeque<(f32, f32)>,
    pending_action: &mut Option<Action>,
    engine: &mut GameEngine,
) {
    avatar_position.advance(dt);
    if !avatar_position.is_settled() {
        return;
    }
    if let Some(next_waypoint) = remaining_path.pop_front() {
        let current = avatar_position.current();
        let leg_distance =
            vec2(current.0, current.1).distance(vec2(next_waypoint.0, next_waypoint.1));
        avatar_position.set_target(next_waypoint, (leg_distance / WALK_SPEED).max(0.05));
    } else if let Some(action) = pending_action.take() {
        let events = engine.execute_action(action);
        for event in events {
            println!("{event:?}");
        }
    }
}

/// Draw every hotspot's sprite (or a plain gray fallback), its hover-aware
/// outline, and its label.
fn draw_hotspots(hotspots: &[SceneHotspot], hovered: Option<usize>) {
    for (i, spot) in hotspots.iter().enumerate() {
        match &spot.sprite {
            Some(texture) => draw_texture_ex(
                texture,
                spot.rect.x,
                spot.rect.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(spot.rect.w, spot.rect.h)),
                    ..Default::default()
                },
            ),
            None => draw_rectangle(spot.rect.x, spot.rect.y, spot.rect.w, spot.rect.h, GRAY),
        }
        let border_color = if hovered == Some(i) { YELLOW } else { GRAY };
        draw_rectangle_lines(
            spot.rect.x,
            spot.rect.y,
            spot.rect.w,
            spot.rect.h,
            2.0,
            border_color,
        );
        draw_text(&spot.label, spot.rect.x, spot.rect.y - 6.0, 16.0, WHITE);
    }
}

/// Draw the current room's walkable-area outline (a faint debug wireframe,
/// since this is a proof of the wiring, not real art) and the placeholder
/// avatar at its current position.
fn draw_walkable_and_avatar(
    walkable: Option<&pathfinding::WalkableArea>,
    avatar_position: (f32, f32),
) {
    if let Some(area) = walkable {
        let boundary = &area.boundary;
        for i in 0..boundary.len() {
            let a = boundary[i];
            let b = boundary[(i + 1) % boundary.len()];
            draw_line(a.0, a.1, b.0, b.1, 1.0, Color::new(1.0, 1.0, 1.0, 0.15));
        }
    }
    draw_circle(avatar_position.0, avatar_position.1, 12.0, ORANGE);
}

/// Draw the radial verb coin over the hovered hotspot (if any), and report
/// the clicked verb - the highest-priority entry - when the player clicks
/// while `can_click` is true (gated on there being no walk already in
/// flight - see `begin_walk_or_execute`/`advance_avatar`'s single
/// `pending_action` slot).
fn draw_verb_coin_and_detect_click(
    hotspots: &[SceneHotspot],
    hovered: Option<usize>,
    engine: &GameEngine,
    coin_alpha: f32,
    can_click: bool,
) -> Option<typegin_core::Verb> {
    let i = hovered?;
    let spot = &hotspots[i];
    let verbs = engine.verbs_for(spot.target.clone());
    let entries = coin::layout(&verbs);
    let center = (
        spot.rect.x + spot.rect.w / 2.0,
        spot.rect.y + spot.rect.h / 2.0,
    );
    let radius = 60.0;
    for entry in &entries {
        let x = center.0 + radius * entry.angle.sin();
        let y = center.1 - radius * entry.angle.cos();
        draw_text(
            &format!("{:?}", entry.verb),
            x,
            y,
            18.0,
            Color::new(1.0, 1.0, 1.0, coin_alpha),
        );
    }
    if is_mouse_button_pressed(MouseButton::Left) && can_click {
        entries.first().map(|entry| entry.verb)
    } else {
        None
    }
}

/// Walk to `destination` before executing `action` (if any) - the
/// authentic `PnC` "walk-then-interact" feel. `action` is `None` for a
/// plain click-to-walk on empty floor (nothing to execute on arrival), and
/// `Some` for a click on a hotspot's verb coin.
///
/// Falls back to instant execution - or, for a plain walk, to doing
/// nothing at all - when there's nowhere to walk: no `gui.walkable`
/// authored for this room yet, or `destination` landed outside it (e.g. on
/// a wall-mounted hotspot like the padding).
fn begin_walk_or_execute(
    action: Option<Action>,
    walkable: Option<&pathfinding::WalkableArea>,
    avatar_position: (f32, f32),
    destination: (f32, f32),
    remaining_path: &mut VecDeque<(f32, f32)>,
    pending_action: &mut Option<Action>,
    engine: &mut GameEngine,
) {
    if let Some(waypoints) =
        walkable.and_then(|area| pathfinding::find_path(area, avatar_position, destination))
    {
        *remaining_path = waypoints.into_iter().collect();
        *pending_action = action;
    } else if let Some(action) = action {
        let events = engine.execute_action(action);
        for event in events {
            println!("{event:?}");
        }
    }
}

/// Translate a chosen verb + target into an `Action`. `Go`/`Examine`/`Talk`
/// map straight onto the target; `Use` here means the scene-object's own
/// self-use (see AGENTS.md's verb-coin design notes: two-object combine
/// always starts from an inventory item, never from a scene hotspot's
/// coin, so `Use` from this coin never carries a second target).
fn action_for(verb: typegin_core::Verb, target: Target) -> Action {
    use typegin_core::Verb;
    match (verb, target) {
        (Verb::Go, Target::Object(id)) => Action::Go(typegin_core::GoTarget::Id(id)),
        (Verb::Examine, target) => Action::Examine(Locator::Id(target)),
        (Verb::Take, Target::Object(id)) => Action::Take(Locator::Id(id)),
        (Verb::Drop, Target::Object(id)) => Action::Drop(Locator::Id(id)),
        (Verb::Talk, Target::Npc(id)) => Action::Talk(Locator::Id(id)),
        (Verb::Use, Target::Object(id)) => Action::Use {
            item: Locator::Id(id),
            target: None,
        },
        (verb, target) => Action::Unknown(format!("{verb:?} on {target:?} (unhandled)")),
    }
}
