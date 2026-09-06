use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

mod view;

fn main() -> ExitCode {
    env_logger::init();

    let data_dir = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data"));
    let items_path = data_dir.join("items.yaml");
    let rooms_path = data_dir.join("rooms.yaml");

    let world_data = match typegin_core::WorldData::load(&items_path, &rooms_path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load world data: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut engine = typegin_core::GameEngine::get_with_rules(&world_data, GameRules);

    let mut view = view::TextView;

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
        for command in typegin_core::View::render(&mut view, &events, engine.world()) {
            match command {
                typegin_core::RenderCommand::Line(text) => println!("{text}"),
                typegin_core::RenderCommand::ClearScreen => {
                    print!("\u{1b}[2J\u{1b}[H");
                    let _ = io::stdout().flush();
                }
                _ => {}
            }
        }
    }

    ExitCode::SUCCESS
}

/// This game's rules: room-specific discovery.
///
/// The core's default `BasicRules` supplies stock behaviour (taking items,
/// refusing to take scene objects, unlocking `gated_by` doors). This game only
/// adds an authored beat: looking around in the study reveals the hidden
/// passage door.
struct GameRules;

impl typegin_core::Rules for GameRules {
    fn on_look(&mut self, world: &mut typegin_core::WorldState) -> Vec<typegin_core::Event> {
        if world.current_room_id() == typegin_core::RoomId::new(3) {
            world.reveal_exit(typegin_core::Direction::North);
        }

        vec![typegin_core::Event::Looked]
    }
}
