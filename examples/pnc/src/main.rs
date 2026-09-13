//! Demonstration that `crates/render` (a reusable rendering engine, no game
//! content of its own — see AGENTS.md) can drive the *same* shared example
//! content `examples/text` and `crates/text-proto` use
//! (`examples/data/`), through a real macroquad window: boot a
//! `GameEngine`, draw the current room's hotspots, show a radial verb coin
//! on hover, and execute the highest-priority verb on click.
//!
//! All the actual logic (hit-testing, coin layout, animation) lives in
//! `render`'s library, not here — this file only wires it together and
//! draws. A real game would replace `candidate_object_ids` below with a
//! proper "list this room's objects" query once core exposes one (see the
//! comment on it) and would own its own asset lookup instead of drawing
//! plain rectangles.

use macroquad::prelude::*;
use typegin_core::{Action, GameEngine, Locator, ObjectId, ObjectResolution, Target, WorldData};

use render::hotspot::{self, Rect};
use render::{coin, tween::Tween};

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
/// this room's objects with their full `ObjectInfo`" query — only names
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
}

/// Every candidate object that's both currently in the room and has
/// authored `gui.hotspot` data. Anything currently in the room *without*
/// hotspot data is returned separately, so it stays visible (as a text
/// fallback) instead of silently vanishing.
fn visible_hotspots(engine: &GameEngine) -> (Vec<SceneHotspot>, Vec<String>) {
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
            Some(rect) => placed.push(SceneHotspot {
                target: Target::Object(id),
                rect,
                label: info.name,
            }),
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
    let mut coin_alpha = Tween::settled(0.0_f32);

    loop {
        clear_background(Color::from_rgba(20, 20, 24, 255));

        let (hotspots, unplaced) = visible_hotspots(&engine);
        let mouse = mouse_position();
        let hovered = hotspots.iter().position(|h| h.rect.contains(mouse));

        coin_alpha.set_target(if hovered.is_some() { 1.0 } else { 0.0 }, 0.15);
        coin_alpha.advance(get_frame_time());

        draw_text(&room_description(&engine), 20.0, 30.0, 20.0, WHITE);

        for (i, spot) in hotspots.iter().enumerate() {
            let color = if hovered == Some(i) { YELLOW } else { GRAY };
            draw_rectangle_lines(
                spot.rect.x,
                spot.rect.y,
                spot.rect.w,
                spot.rect.h,
                2.0,
                color,
            );
            draw_text(&spot.label, spot.rect.x, spot.rect.y - 6.0, 16.0, WHITE);
        }

        if !unplaced.is_empty() {
            draw_text(
                &format!("(no hotspot authored yet: {})", unplaced.join(", ")),
                20.0,
                screen_height() - 20.0,
                16.0,
                GRAY,
            );
        }

        let mut clicked_verb = None;
        if let Some(i) = hovered {
            let spot = &hotspots[i];
            let verbs = engine.verbs_for(spot.target.clone());
            let entries = coin::layout(&verbs);
            let center = (
                spot.rect.x + spot.rect.w / 2.0,
                spot.rect.y + spot.rect.h / 2.0,
            );
            let radius = 60.0;
            let alpha = coin_alpha.current();
            for entry in &entries {
                let x = center.0 + radius * entry.angle.sin();
                let y = center.1 - radius * entry.angle.cos();
                draw_text(
                    &format!("{:?}", entry.verb),
                    x,
                    y,
                    18.0,
                    Color::new(1.0, 1.0, 1.0, alpha),
                );
            }
            if is_mouse_button_pressed(MouseButton::Left) {
                clicked_verb = entries.first().map(|entry| entry.verb);
            }
        }

        if let Some(verb) = clicked_verb {
            let target = hotspots[hovered.expect("clicked_verb only set while hovered")]
                .target
                .clone();
            let action = action_for(verb, target);
            let events = engine.execute_action(action);
            for event in events {
                println!("{event:?}");
            }
        }

        next_frame().await;
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
