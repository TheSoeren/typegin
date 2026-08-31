use std::env;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::ExitCode;

use crate::view::TextView;
use typegin_core::{Event, GameEngine, Resolution, Rules, View, WorldData, WorldState};

mod view;

fn main() -> ExitCode {
    env_logger::init();

    let data_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    let items_path = data_dir.join("items.toml");
    let rooms_path = data_dir.join("rooms.toml");

    let world_data = match WorldData::load(&items_path, &rooms_path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load world data: {err}");
            return ExitCode::FAILURE;
        }
    };

    let db_path = env::var("TYPEGIN_DB").unwrap_or_else(|_| "typegin.db".to_string());

    let mut engine = match GameEngine::open_with_rules(&db_path, &world_data, TakeRules) {
        Ok(engine) => engine,
        Err(err) => {
            eprintln!("Failed to open game database: {err}");
            return ExitCode::FAILURE;
        }
    };

    let view = TextView;

    println!("You find yourself in a mysterious place.");
    println!("Type 'look' to see where you are. 'quit' to leave.\n");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let input = match line {
            Ok(input) => input.trim().to_string(),
            Err(_) => break,
        };

        if input.is_empty() {
            continue;
        }
        if input == "quit" || input == "exit" {
            break;
        }

        let events = engine.handle_input(&input);
        for line in view.render(&events, &engine.world) {
            println!("{line}");
        }
    }

    ExitCode::SUCCESS
}

/// This game's rules: how the player actually picks items up.
///
/// The core's default `BasicRules` is intentionally inert — moving items
/// between the room and the inventory is behaviour supplied by the game.
struct TakeRules;

impl Rules for TakeRules {
    fn on_take(
        &mut self,
        world: &mut WorldState,
        name: &str,
        resolution: Resolution,
    ) -> Vec<Event> {
        match resolution {
            Resolution::Found(id) => {
                if world.player_has_item(id) {
                    match world.item_info(id) {
                        Some(info) => return vec![Event::AlreadyHolding { item: info.name }],
                        None => {
                            return vec![Event::Message(
                                "You are already holding that.".to_string(),
                            )];
                        }
                    }
                }

                if world.move_item_to_inventory(id) {
                    let item = world
                        .item_info(id)
                        .map(|info| info.name)
                        .unwrap_or_else(|| name.to_string());
                    vec![Event::Took { item }]
                } else {
                    vec![Event::Message("Not in room.".to_string())]
                }
            }
            Resolution::Ambiguous(_) => vec![Event::Ambiguous {
                phrase: name.to_string(),
            }],
            Resolution::NotFound => vec![Event::NotFound {
                phrase: name.to_string(),
            }],
        }
    }
}
