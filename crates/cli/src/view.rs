use typegin_core::input::GoTarget;
use typegin_core::{Event, WorldState};

/// Default player-facing wording for the game.
///
/// Keeping the wording here (instead of in the engine) means you can change
/// every sentence in the game without touching game logic. `typegin_core`
/// has no rendering contract of its own (no `View` trait) — this is just a
/// plain function over `Event`, the simplest thing that works for a single
/// first-party text front-end.
///
/// Only the events matched here produce output; anything else silently
/// produces nothing, so a new engine `Event` never breaks this front-end —
/// it just needs an arm added here when you want it voiced.
pub fn render(events: &[Event], world: &WorldState) -> Vec<String> {
    events
        .iter()
        .flat_map(|event| render_event(event, world))
        .collect()
}

fn render_event(event: &Event, world: &WorldState) -> Vec<String> {
    match event {
        Event::Looked => render_look(world),
        Event::Went(go_target) => vec![format!("You go {go_target:?}.")],
        Event::WentExitNotFound(go_target) => {
            vec![format!("You can't go that way ({go_target:?}).")]
        }
        Event::WentExitHidden(go_target) => vec![format!("The {go_target:?} door is hidden.")],
        Event::WentExitLocked(go_target) => vec![format!("The {go_target:?} door is locked.")],
        Event::UnlockedExit(go_target) => vec![format!("The {go_target:?} door swings open.")],
        Event::CannotUse { item, target } => vec![format!(
            "That doesn't work with the {item} on the {target}."
        )],
        Event::Custom { name } => vec![name.clone()],
        Event::Took { object, .. } => vec![format!("You take the {object}.")],
        Event::TookObjectNotFound { object } => vec![format!("I don't see any {object} here.")],
        Event::CantTake { object } => vec![format!("You can't carry the {object}.")],
        Event::Dropped { object, .. } => vec![format!("You dropped the {object}.")],
        Event::DroppedObjectNotFound { object } => {
            vec![format!("You aren't carrying any {object}.")]
        }
        Event::Used { object, target, .. } => vec![match target {
            Some(target) => format!("You use the {object} on the {target}."),
            None => format!("You use the {object}."),
        }],
        Event::UsedObjectNotFound { object } => vec![format!("You don't have a {object}.")],
        Event::UsedTargetNeeded { object, .. } => {
            vec![format!("You need to use the {object} on something.")]
        }
        Event::UsedTargetNotFound { object, target, .. } => {
            vec![format!("You can't use the {object} on {target}.")]
        }
        Event::UsedTargetAmbiguous { object, .. } => {
            vec![format!("Which target do you want to use the {object} on?")]
        }
        Event::Examined {
            target_name: object,
            ..
        } => vec![format!("You examine the {object}.")],
        Event::ExaminedTargetNotFound { target: object } => vec![format!("There is no {object}.")],
        Event::TookObjectAmbiguous { object, .. }
        | Event::DroppedObjectAmbiguous { object, .. }
        | Event::UsedObjectAmbiguous { object, .. }
        | Event::ExaminedTargetAmbiguous { target: object, .. } => {
            vec![format!("Which {object} do you mean? Be more specific.")]
        }
        Event::UnknownEvent { name } => vec![format!("I don't understand \"{name}\".")],
        Event::Talked {
            npc, text, choices, ..
        } => {
            let mut out = vec![format!("{npc}: {text}")];
            for (i, choice) in choices.iter().enumerate() {
                out.push(format!("  {}. {}", i + 1, choice.label));
            }
            out
        }
        Event::DialogueEnded { npc, .. } => vec![format!("({npc} falls silent.)")],
        Event::TalkNpcNotFound { npc } => vec![format!("There is no {npc} here.")],
        Event::DialogueInvalidChoice { choice, .. } => {
            vec![format!("\"{choice}\" isn't an option.")]
        }
        _ => Vec::new(),
    }
}

fn render_look(world: &WorldState) -> Vec<String> {
    let room_items = world.room_object_names();
    let inventory = world.player_object_names();

    let mut lines = vec![room_description(world)];

    if room_items.is_empty() {
        lines.push("There is nothing notable here.".to_string());
    } else {
        lines.push(format!("You can see: {}.", join_list(&room_items)));
    }

    if inventory.is_empty() {
        lines.push("You are carrying nothing.".to_string());
    } else {
        lines.push(format!("You are carrying: {}.", join_list(&inventory)));
    }

    let exits = visible_exits(world);
    if exits.is_empty() {
        lines.push("There are no visible exits here.".to_string());
    } else {
        lines.push(format!("Exits: {}.", join_list(&exits)));
    }

    lines
}

fn visible_exits(world: &WorldState) -> Vec<String> {
    world
        .exit_directions()
        .into_iter()
        .filter_map(|direction| {
            if world.is_exit_hidden(&GoTarget::Direction(direction)) {
                return None;
            }
            let exit = world.exit_info(&GoTarget::Direction(direction))?;
            let state = if world.is_exit_locked(&GoTarget::Direction(direction)) {
                " (locked)"
            } else {
                ""
            };
            Some(format!("{direction} through the {}{state}", exit.name))
        })
        .collect()
}

fn room_description(world: &WorldState) -> String {
    match world
        .current_room_extra()
        .get("description")
        .map(typegin_core::ExtraValue::to_owned)
    {
        Some(typegin_core::ExtraValue::Str(desc)) => desc,
        _ => "You are in a room.".to_string(),
    }
}

fn join_list(list: &[String]) -> String {
    match list {
        [] => String::new(),
        [single] => single.clone(),
        [first, rest @ ..] => {
            let mut out = first.clone();
            for (i, item) in rest.iter().enumerate() {
                if i == rest.len() - 1 {
                    out.push_str(", and ");
                } else {
                    out.push_str(", ");
                }
                out.push_str(item);
            }
            out
        }
    }
}
