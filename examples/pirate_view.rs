// Demonstrates a custom `Rules` object and a custom `View` — showing that the
// engine's behaviour, wording, and rendering are all swappable extension points
// rather than baked into the core, with no wrapper struct required.
use std::io::{self, BufRead};
use std::process::ExitCode;

use typegin_core::{BasicRules, Event, GameEngine, Resolution, Rules, View, WorldData, WorldState};

fn main() -> ExitCode {
    let data = WorldData::load("data/items.toml", "data/rooms.toml").unwrap();
    let mut engine = GameEngine::open_with_rules("flavor_example.db", &data, FlavorRules).unwrap();
    let view = PirateView;

    println!("The salty sea air greets ye... ('quit' to leave)\n");

    let stdin = io::stdin();
    for line in stdin.lock().lines().flatten() {
        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        if input == "quit" {
            break;
        }

        let events = engine.handle_input(input);
        for line in view.render(&events, &engine.world) {
            println!("{line}");
        }
    }

    let _ = std::fs::remove_file("flavor_example.db");
    ExitCode::SUCCESS
}

// --- Custom rules: a behaviour rule injected into the engine -----------------
struct FlavorRules;

impl Rules for FlavorRules {
    // The brass key (id 4) won't come free without a cutlass.
    fn on_take(&mut self, world: &mut WorldState, name: &str, resolution: Resolution) -> Vec<Event> {
        if matches!(resolution, Resolution::Found(4)) {
            vec![Event::Message(
                "The brass key is chained to the wall — a sturdy cutlass might free it.".to_string(),
            )]
        } else {
            BasicRules.on_take(world, name, resolution)
        }
    }
}

// --- Custom View: completely different wording/rendering ----------------------
struct PirateView;

impl View for PirateView {
    fn render(&self, events: &[Event], _world: &WorldState) -> Vec<String> {
        events
            .iter()
            .map(|event| self.render_event(event))
            .collect()
    }
}

impl PirateView {
    fn render_event(&self, event: &Event) -> String {
        match event {
            Event::Looked => "Ye cast yer eyes about the room.".to_string(),
            Event::Went(direction) => format!("Ye set sail {:?}.", direction),
            Event::Took { item } => format!("Ye grab the {item}, and stow it away."),
            Event::Used { item, target } => match target {
                Some(target) => format!("Ye jab the {item} at the {target}."),
                None => format!("Ye fiddle with the {item}."),
            },
            Event::AlreadyHolding { item } => format!("Ye already carry the {item}, ye pack rat."),
            Event::NotFound { phrase } => format!("Yar? No {phrase} here, matey."),
            Event::Ambiguous { phrase } => format!("Too many {phrase}s about, ye scallywag."),
            Event::Message(text) => text.clone(),
        }
    }
}
