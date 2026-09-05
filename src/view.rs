/// Default player-facing wording for the game.
///
/// Keeping the wording here (instead of in the engine) means you can change
/// every sentence in the game without touching game logic, or provide your
/// own `View` for custom flavour.
pub struct TextView;

impl typegin_core::View for TextView {
    fn render(
        &self,
        events: &[typegin_core::Event],
        world: &typegin_core::WorldState,
    ) -> Vec<String> {
        events
            .iter()
            .flat_map(|event| self.render_event(event, world))
            .collect()
    }
}

impl TextView {
    fn render_event(
        &self,
        event: &typegin_core::Event,
        world: &typegin_core::WorldState,
    ) -> Vec<String> {
        let line = match event {
            typegin_core::Event::Looked => return render_look(world),
            typegin_core::Event::Went(direction) => {
                format!("You go {:?}.", direction)
            }
            typegin_core::Event::WentInvalidDirection(direction) => {
                format!("You can't go that way ({:?}).", direction)
            }
            typegin_core::Event::Took { item_id: _, item } => {
                format!("You take the {item}.")
            }
            typegin_core::Event::TookItemNotFound { item } => {
                format!("I don't see any {item} here.")
            }
            typegin_core::Event::TookItemAmbiguous { item_ids: _, item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            typegin_core::Event::Dropped { item_id: _, item } => {
                format!("You dropped the {item}.")
            }
            typegin_core::Event::DroppedItemNotFound { item } => {
                format!("You aren't carrying any {item}.")
            }
            typegin_core::Event::DroppedItemAmbiguous { item_ids: _, item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            typegin_core::Event::Used {
                item_id: _,
                item,
                target_id: _,
                target,
            } => match target {
                Some(target) => format!("You use the {item} on the {target}."),
                None => format!("You use the {item}."),
            },
            typegin_core::Event::UsedItemNotFound { item } => {
                format!("You don't have a {item}.")
            }
            typegin_core::Event::UsedItemAmbiguous { item_ids: _, item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            typegin_core::Event::UsedTargetNeeded { item_id: _, item } => {
                format!("You need to use the {item} on something.")
            }
            typegin_core::Event::UsedTargetNotFound {
                item_id: _,
                item,
                target,
            } => {
                format!("You can't use the {item} on {target}.")
            }
            typegin_core::Event::UsedTargetAmbiguous {
                item_id: _,
                item,
                target_ids: _,
                target: _,
            } => {
                format!("Which target do you want to use the {item} on?")
            }
            typegin_core::Event::Examined { item_id: _, item } => {
                format!("You examine the {item}.")
            }
            typegin_core::Event::ExaminedItemNotFound { item } => {
                format!("There is no {item}.")
            }
            typegin_core::Event::ExaminedItemAmbiguous { item_ids: _, item } => {
                format!("Which {item} do you mean? Be more specific.")
            }
            typegin_core::Event::UnknownEvent { name } => {
                format!("I don't understand \"{name}\".")
            }
        };
        vec![line]
    }
}

fn render_look(world: &typegin_core::WorldState) -> Vec<String> {
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
