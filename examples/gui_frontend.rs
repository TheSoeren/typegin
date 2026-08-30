// Demonstrates a GUI-style front-end that consumes the engine's typed
// `Event`s and `WorldState` directly. It never touches the `View` trait —
// proving that output style is fully decoupled from game logic and can be
// swapped freely (text UI, GUI, web, ... all read the same engine).
use std::io::{self, BufRead};
use std::process::ExitCode;

use typegin_core::{Direction, Event, GameEngine, WorldData};

fn main() -> ExitCode {
    let data = WorldData::load("data/items.toml", "data/rooms.toml").unwrap();
    let mut engine = GameEngine::open("gui_example.db", &data).unwrap();

    println!("=== GUI FRONT-END DEMO (no View trait used) ===");

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

        // A GUI would push these into widgets; here we fake it with text to
        // keep the example self-contained. The key point: we receive typed
        // events, not pre-rendered strings, so a real GUI can branch on them.
        println!("\n-- UI state after command '{input}' --");
        for event in &events {
            match event {
                Event::Looked => {
                    println!("[render room] room items: {}", engine.world.room_item_names().join(", "));
                    println!("[render inv ] inventory: {}", engine.world.inventory_item_names().join(", "));
                }
                Event::Went(direction) => print_gui(direction),
                Event::Took { item } => {
                    println!("[inventory panel] + {item}");
                    println!("[update room list ] {}", engine.world.room_item_names().join(", "));
                }
                Event::Used { item, target } => println!("[effect panel] used {item} -> {target:?}"),
                Event::AlreadyHolding { item } => println!("[toast] already holding {item}"),
                Event::NotFound { phrase } => println!("[input feedback] not found: '{phrase}'"),
                Event::Ambiguous { phrase } => {
                    println!("[input feedback] ambiguous: '{phrase}'")
                }
                Event::Message(text) => println!("[message panel] {text}"),
            }
        }
    }

    let _ = std::fs::remove_file("gui_example.db");
    ExitCode::SUCCESS
}

fn print_gui(direction: &Direction) {
    use Direction::*;
    let arrow = match direction {
        North => "N",
        South => "S",
        East => "E",
        West => "W",
    };
    println!("[compass] player moved {arrow}");
}
