use std::env;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

mod view;

fn main() -> ExitCode {
    env_logger::init();

    let data_dir = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("data"), PathBuf::from);
    let globals_path = data_dir.join("globals.yaml");
    let items_path = data_dir.join("items.yaml");
    let rooms_path = data_dir.join("rooms.yaml");
    let interactions_path = data_dir.join("interactions.yaml");
    let npcs_path = data_dir.join("npcs.yaml");

    let world_data = match typegin_core::WorldData::load(
        &globals_path,
        &items_path,
        &rooms_path,
        &interactions_path,
        &npcs_path,
    ) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load world data: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut engine = typegin_core::GameEngine::get(&world_data);

    let mut view = view::TextView;

    println!(
        "You wake up in a padded cell of the sanatorium. Harvey is in your hands, chin tucked against you."
    );
    println!("Type 'look' to see where you are, 'talk to harvey' to speak. 'quit' to leave.\n");

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
