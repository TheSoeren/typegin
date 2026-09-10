/// Default player-facing wording for the game.
///
/// Keeping the wording here (instead of in the engine) means you can change
/// every sentence in the game without touching game logic, or provide your
/// own `View` for custom flavour.
///
/// Only events it phrases are overridden; an engine event nobody has written
/// prose for yet silently produces no output, so a future event never breaks
/// this view.
pub struct TextView;

impl typegin_core::View for TextView {
    fn render_looked(
        &mut self,
        world: &typegin_core::WorldState,
    ) -> Vec<typegin_core::RenderCommand> {
        render_look(world)
    }

    fn render_went(
        &mut self,
        direction: &typegin_core::Direction,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You go {direction:?}."))]
    }

    fn render_went_invalid_direction(
        &mut self,
        direction: &typegin_core::Direction,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You can't go that way ({direction:?})."))]
    }

    fn render_went_exit_hidden(
        &mut self,
        direction: &typegin_core::Direction,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("The {direction:?} door is hidden."))]
    }

    fn render_went_exit_locked(
        &mut self,
        direction: &typegin_core::Direction,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("The {direction:?} door is locked."))]
    }

    fn render_unlocked_exit(
        &mut self,
        direction: &typegin_core::Direction,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("The {direction:?} door swings open."))]
    }

    fn render_cannot_use(&mut self, item: &str, target: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!(
            "That doesn't work with the {item} on the {target}."
        ))]
    }

    fn render_custom(&mut self, name: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(name.to_string())]
    }

    fn render_took(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You take the {object}."))]
    }

    fn render_took_object_not_found(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("I don't see any {object} here."))]
    }

    fn render_took_object_ambiguous(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!(
            "Which {object} do you mean? Be more specific."
        ))]
    }

    fn render_cant_take(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You can't carry the {object}."))]
    }

    fn render_dropped(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You dropped the {object}."))]
    }

    fn render_dropped_object_not_found(
        &mut self,
        object: &str,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You aren't carrying any {object}."))]
    }

    fn render_dropped_object_ambiguous(
        &mut self,
        object: &str,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!(
            "Which {object} do you mean? Be more specific."
        ))]
    }

    fn render_used(
        &mut self,
        object: &str,
        target: Option<&str>,
    ) -> Vec<typegin_core::RenderCommand> {
        let text = match target {
            Some(target) => format!("You use the {object} on the {target}."),
            None => format!("You use the {object}."),
        };
        vec![line(text)]
    }

    fn render_used_object_not_found(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You don't have a {object}."))]
    }

    fn render_used_object_ambiguous(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!(
            "Which {object} do you mean? Be more specific."
        ))]
    }

    fn render_used_target_needed(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You need to use the {object} on something."))]
    }

    fn render_used_target_not_found(
        &mut self,
        object: &str,
        target: &str,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You can't use the {object} on {target}."))]
    }

    fn render_used_target_ambiguous(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!(
            "Which target do you want to use the {object} on?"
        ))]
    }

    fn render_examined(&mut self, object: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("You examine the {object}."))]
    }

    fn render_examined_object_not_found(
        &mut self,
        object: &str,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("There is no {object}."))]
    }

    fn render_examined_object_ambiguous(
        &mut self,
        object: &str,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!(
            "Which {object} do you mean? Be more specific."
        ))]
    }

    fn render_unknown_event(&mut self, name: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("I don't understand \"{name}\"."))]
    }

    fn render_talked(
        &mut self,
        npc: &str,
        text: &str,
        choices: &[typegin_core::event::DialogueChoice],
    ) -> Vec<typegin_core::RenderCommand> {
        let mut out = vec![line(format!("{npc}: {text}"))];
        for (i, choice) in choices.iter().enumerate() {
            out.push(line(format!("  {}. {}", i + 1, choice.label)));
        }
        out
    }

    fn render_dialogue_ended(&mut self, npc: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("({npc} falls silent.)"))]
    }

    fn render_talk_npc_not_found(&mut self, npc: &str) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("There is no {npc} here."))]
    }

    fn render_dialogue_invalid_choice(
        &mut self,
        _npc: &str,
        choice: &str,
    ) -> Vec<typegin_core::RenderCommand> {
        vec![line(format!("\"{choice}\" isn't an option."))]
    }
}

fn line(text: String) -> typegin_core::RenderCommand {
    typegin_core::RenderCommand::Line(text)
}

fn render_look(world: &typegin_core::WorldState) -> Vec<typegin_core::RenderCommand> {
    let room_items = world.room_object_names();
    let inventory = world.player_object_names();

    let mut parts = vec![line(room_description(world))];

    if room_items.is_empty() {
        parts.push(line("There is nothing notable here.".to_string()));
    } else {
        let items = join_list(&room_items);
        parts.push(line(format!("You can see: {items}.")));
    }

    if inventory.is_empty() {
        parts.push(line("You are carrying nothing.".to_string()));
    } else {
        let carried = join_list(&inventory);
        parts.push(line(format!("You are carrying: {carried}.")));
    }

    let exits = visible_exits(world);
    if exits.is_empty() {
        parts.push(line("There are no visible exits here.".to_string()));
    } else {
        let listed = join_list(&exits);
        parts.push(line(format!("Exits: {listed}.")));
    }

    // Yield one line per sentence so each is a distinct message.
    parts
}

fn visible_exits(world: &typegin_core::WorldState) -> Vec<String> {
    const COMPASS: [typegin_core::Direction; 4] = [
        typegin_core::Direction::North,
        typegin_core::Direction::East,
        typegin_core::Direction::South,
        typegin_core::Direction::West,
    ];

    COMPASS
        .into_iter()
        .filter_map(|direction| {
            if world.is_exit_hidden(direction) {
                return None;
            }
            let exit = world.exit_info(direction)?;
            let state = if world.is_exit_locked(direction) {
                " (locked)"
            } else {
                ""
            };
            Some(format!("{direction} through the {}{state}", exit.name))
        })
        .collect()
}

fn room_description(world: &typegin_core::WorldState) -> String {
    match world
        .current_room_extra()
        .get("description")
        .map(typegin_core::data::ExtraValue::to_owned)
    {
        Some(typegin_core::data::ExtraValue::Str(desc)) => desc,
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
