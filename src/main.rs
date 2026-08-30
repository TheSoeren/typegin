use std::env;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::process::ExitCode;

use typegin_core::{GameEngine, WorldData};

fn main() -> ExitCode {
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

    let mut engine = match GameEngine::open(&db_path, &world_data) {
        Ok(engine) => engine,
        Err(err) => {
            eprintln!("Failed to open game database: {err}");
            return ExitCode::FAILURE;
        }
    };

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

        println!("{}", engine.handle_input(&input));
    }

    ExitCode::SUCCESS
}

