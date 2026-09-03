use std::env;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::ExitCode;

mod view;

fn main() -> ExitCode {
    env_logger::init();

    let data_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    let items_path = data_dir.join("items.toml");
    let rooms_path = data_dir.join("rooms.toml");

    let world_data = match typegin_core::WorldData::load(&items_path, &rooms_path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load world data: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut engine = typegin_core::GameEngine::get_with_rules(&world_data, TakeRules);

    let view = view::TextView;

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
        for line in typegin_core::View::render(&view, &events, engine.world()) {
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

impl typegin_core::Rules for TakeRules {
    fn on_take(
        &mut self,
        world: &mut typegin_core::WorldState,
        name: &str,
        resolution: typegin_core::ItemResolution,
    ) -> Vec<typegin_core::Event> {
        match resolution {
            typegin_core::ItemResolution::Found(id) => match world.player_take_item(id) {
                typegin_core::TakeResult::Success => {
                    vec![typegin_core::Event::Took {
                        item: name.to_string(),
                    }]
                }
                typegin_core::TakeResult::Fail => {
                    vec![typegin_core::Event::TookItemNotFound {
                        item: name.to_string(),
                    }]
                }
            },
            typegin_core::ItemResolution::Ambiguous(_) => {
                vec![typegin_core::Event::TookItemAmbiguous {
                    item: name.to_string(),
                }]
            }
            typegin_core::ItemResolution::NotFound => {
                vec![typegin_core::Event::TookItemNotFound {
                    item: name.to_string(),
                }]
            }
        }
    }

    fn on_look(&mut self, world: &mut typegin_core::WorldState) -> Vec<typegin_core::Event> {
        if world.current_room_id() == typegin_core::RoomId::new(3) {
            world.reveal_exit(typegin_core::Direction::North);
        }

        vec![typegin_core::Event::Looked]
    }
}
