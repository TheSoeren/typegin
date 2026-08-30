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
            Event::Took { item } => format!("You take the {item}."),
            Event::Used { item, target } => match target {
                Some(target) => format!("You use the {item} on the {target}."),
                None => format!("You use the {item}."),
            },
            Event::AlreadyHolding { item } => format!("You are already holding the {item}."),
            Event::NotFound { phrase } => format!("I don't see any {phrase} here."),
            Event::Ambiguous { phrase } => {
                format!("Which {phrase} do you mean? Be more specific.")
            }
            Event::Message(text) => text.clone(),
        };
        vec![line]
    }
}

fn render_look(world: &WorldState) -> Vec<String> {
    let room_items = world.room_item_names();
    let inventory = world.inventory_item_names();

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

#[cfg(test)]
mod tests {
    use super::*;
    use typegin_core::{GameEngine, WorldData};

    fn dummy_world() -> WorldState {
        let data = WorldData::from_toml(
            include_str!("../data/items.toml"),
            include_str!("../data/rooms.toml"),
        )
        .expect("parse world data");
        GameEngine::open(":memory:", &data)
            .expect("open game")
            .world
    }

    #[test]
    fn joins_list_with_oxford_comma() {
        assert_eq!(join_list(&["a".to_string()]), "a");
        assert_eq!(join_list(&["a".to_string(), "b".to_string()]), "a, and b");
        assert_eq!(
            join_list(&["a".to_string(), "b".to_string(), "c".to_string()]),
            "a, b, and c"
        );
    }

    #[test]
    fn renders_took_event() {
        let view = TextView;
        let out = view.render(
            &[Event::Took {
                item: "iron key".to_string(),
            }],
            &dummy_world(),
        );
        assert_eq!(out, vec!["You take the iron key."]);
    }
}

