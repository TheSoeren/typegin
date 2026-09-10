use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod view;

/// The engine only ever parses one YAML document, so the shipped
/// `data/*.yaml` files (kept separate purely for authoring convenience) are
/// concatenated here before handing the result to
/// [`typegin_core::WorldData::from_yaml`] — each file contributes disjoint
/// top-level keys, so concatenation is a lossless merge.
fn read_world_yaml(data_dir: &Path) -> Result<String, typegin_core::WorldDataError> {
    let mut yaml = String::new();
    for file_name in [
        "globals.yaml",
        "items.yaml",
        "rooms.yaml",
        "interactions.yaml",
        "npcs.yaml",
    ] {
        yaml.push_str(&std::fs::read_to_string(data_dir.join(file_name))?);
        yaml.push('\n');
    }
    Ok(yaml)
}

fn main() -> ExitCode {
    env_logger::init();

    let data_dir = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("data"), PathBuf::from);

    let world_data = match read_world_yaml(&data_dir)
        .and_then(|yaml| typegin_core::WorldData::from_yaml(&yaml))
    {
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
