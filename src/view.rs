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
            typegin_core::Event::WentExitHidden(direction) => {
                format!("The {direction:?} door is hidden.")
            }
            typegin_core::Event::WentExitLocked(direction) => {
                format!("The {direction:?} door is locked.")
            }
            typegin_core::Event::UnlockedExit { direction } => {
                format!("The {direction:?} door swings open.")
            }
            typegin_core::Event::CannotUse { item, target } => {
                format!("That doesn't work with the {item} on the {target}.")
            }
            typegin_core::Event::Custom { name } => format!("({name})"),
            typegin_core::Event::Took {
                object_id: _,
                object,
            } => {
                format!("You take the {object}.")
            }
            typegin_core::Event::TookObjectNotFound { object } => {
                format!("I don't see any {object} here.")
            }
            typegin_core::Event::TookObjectAmbiguous {
                object_ids: _,
                object,
            } => {
                format!("Which {object} do you mean? Be more specific.")
            }
            typegin_core::Event::CantTake { object } => {
                format!("You can't carry the {object}.")
            }
            typegin_core::Event::Dropped {
                object_id: _,
                object,
            } => {
                format!("You dropped the {object}.")
            }
            typegin_core::Event::DroppedObjectNotFound { object } => {
                format!("You aren't carrying any {object}.")
            }
            typegin_core::Event::DroppedObjectAmbiguous {
                object_ids: _,
                object,
            } => {
                format!("Which {object} do you mean? Be more specific.")
            }
            typegin_core::Event::Used {
                object_id: _,
                object,
                target_id: _,
                target,
            } => match target {
                Some(target) => format!("You use the {object} on the {target}."),
                None => format!("You use the {object}."),
            },
            typegin_core::Event::UsedObjectNotFound { object } => {
                format!("You don't have a {object}.")
            }
            typegin_core::Event::UsedObjectAmbiguous {
                object_ids: _,
                object,
            } => {
                format!("Which {object} do you mean? Be more specific.")
            }
            typegin_core::Event::UsedTargetNeeded {
                object_id: _,
                object,
            } => {
                format!("You need to use the {object} on something.")
            }
            typegin_core::Event::UsedTargetNotFound {
                object_id: _,
                object,
                target,
            } => {
                format!("You can't use the {object} on {target}.")
            }
            typegin_core::Event::UsedTargetAmbiguous {
                object_id: _,
                object,
                target_ids: _,
                target: _,
            } => {
                format!("Which target do you want to use the {object} on?")
            }
            typegin_core::Event::Examined {
                object_id: _,
                object,
            } => {
                format!("You examine the {object}.")
            }
            typegin_core::Event::ExaminedObjectNotFound { object } => {
                format!("There is no {object}.")
            }
            typegin_core::Event::ExaminedObjectAmbiguous {
                object_ids: _,
                object,
            } => {
                format!("Which {object} do you mean? Be more specific.")
            }
            typegin_core::Event::UnknownEvent { name } => {
                format!("I don't understand \"{name}\".")
            }
        };
        vec![line]
    }
}

fn render_look(world: &typegin_core::WorldState) -> Vec<String> {
    let room_items = world.room_object_names();
    let inventory = world.player_object_names();

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
