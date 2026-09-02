use typegin_core::{Event, View, WorldState};

/// Default player-facing wording for the game.
///
/// Keeping the wording here (instead of in the engine) means you can change
/// every sentence in the game without touching game logic, or provide your
/// own `View` for custom flavour.
pub struct TextView;

impl View for TextView {
    fn render(&self, events: &[Event], world: &WorldState) -> Vec<String> {
        events
            .iter()
            .flat_map(|event| self.render_event(event, world))
            .collect()
    }
}

impl TextView {
    fn render_event(&self, event: &Event, world: &WorldState) -> Vec<String> {
        let line = match event {
            Event::Looked => return render_look(world),
            Event::Went(direction) => format!("You go {:?}.", direction),
            Event::WentInvalidDirection(direction) => {
                format!("You can't go that way ({:?}).", direction)
            }
            Event::Took { item } => format!("You take the {item}."),
            Event::TookItemNotFound { item } => format!("I don't see any {item} here."),
            Event::TookItemAmbiguous { item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            Event::Dropped { item } => format!("You dropped the {item}."),
            Event::DroppedItemNotFound { item } => {
                format!("You aren't carrying any {item}.")
            }
            Event::DroppedItemAmbiguous { item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            Event::Used { item, target } => match target {
                Some(target) => format!("You use the {item} on the {target}."),
                None => format!("You use the {item}."),
            },
            Event::UsedItemNotFound { item } => format!("You don't have a {item}."),
            Event::UsedItemAmbiguous { item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            Event::UsedTargetNeeded { item } => {
                format!("You need to use the {item} on something.")
            }
            Event::UsedTargetNotFound { item, target } => {
                format!("You can't use the {item} on {target}.")
            }
            Event::UsedTargetAmbiguous { item } => {
                format!("Which target do you want to use the {item} on?")
            }
            Event::Examined { item } => format!("You examine the {item}."),
            Event::ExaminedItemNotFound { item } => format!("There is no {item}."),
            Event::ExaminedItemAmbiguous { item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            Event::UnknownEvent { name } => format!("I don't understand \"{name}\"."),
        };
        vec![line]
    }
}

fn render_look(world: &WorldState) -> Vec<String> {
    let room_items = world.room_item_names();
    let inventory = world.player_item_names();

    let mut parts = vec!["You are in a room.".to_string()];

    if room_items.is_empty() {
        parts.push("There is nothing notable here.".to_string());
    } else {
        let items = join_list(&room_items);
        parts.push(format!("You can see: {items}."));
    }

    if inventory.is_empty() {
        parts.push("You are carrying nothing.".to_string());
    } else {
        let carried = join_list(&inventory);
        parts.push(format!("You are carrying: {carried}."));
    }

    // Yield one line per sentence so each is a distinct message.
    parts
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
