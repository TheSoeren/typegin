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
        vec![line(format!("({name})"))]
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
}

fn line(text: String) -> typegin_core::RenderCommand {
    typegin_core::RenderCommand::Line(text)
}

fn render_look(world: &typegin_core::WorldState) -> Vec<typegin_core::RenderCommand> {
    let room_items = world.room_object_names();
    let inventory = world.player_object_names();

    let mut parts = vec![line("You are in a room.".to_string())];

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
