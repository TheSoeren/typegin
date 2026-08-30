# typegin

A small text-adventure engine in Rust. The core is a pure game state +
persistence layer that turns typed commands into structured `Event`s; any
front-end (terminal, GUI, web, ...) can render those events however it likes.

## Concepts

The engine is **MVC with a passive view**, wired up with two extension seams:

| Piece      | File                                         | Role                                        |
| ---------- | -------------------------------------------- | ------------------------------------------- |
| Model      | [`WorldState`](crates/core/src/world/mod.rs) | Rooms, items, inventory, player position    |
| Controller | [`GameEngine`](crates/core/src/engine.rs)    | Parses input -> `Action`, mutates the model |
| Output     | [`Event`](crates/core/src/event.rs)          | Typed, lossless result of an action         |
| View       | [`View`](crates/core/src/view.rs)            | Trait that renders events + world to text   |

- **`Rules`** (`engine.rs`) is the _inbound_ hook. Inject it via
  `GameEngine::open_with_rules` to override behaviour - examine an item
  differently, veto a `take`, run custom logic. It is a Strategy plugged into
  the controller.
- **`View`** is the _outbound_ hook. It only ever observes `Event`s and a shared
  `&WorldState` - it can never mutate the game. Consume events directly and
  skip views entirely (`examples/gui_frontend.rs`), or swap prose
  (`examples/pirate_view.rs`).

## Quick start

```sh
cargo run
```

Type `look` to see the room, `quit` to leave. The database is created at
`typegin.db` (override with the `TYPEGIN_DB` env var).

### Commands

| Command                  | Shorthand | Meaning                        |
| ------------------------ | --------- | ------------------------------ |
| `look`                   | `l`       | Describe the current room      |
| `go north`               | `n`       | Move in a direction (`e/s/w`)  |
| `examine <thing>`        | `x`       | Inspect an item                |
| `take <item>`            | `get`     | Pick up an item into inventory |
| `use <item>`             |           | Use an item                    |
| `use <item> on <target>` |           | Use an item on a target        |
| `quit` / `exit`          |           | Leave the game                 |

## Layout

```
crates/core/        # engine library: state, input parsing, persistence, rules, view
  src/
    engine.rs       # GameEngine + Rules trait + BasicRules
    world/          # WorldState, items, rooms, player
    event.rs        # Event enum
    input/          # tokenizer + lexer: text -> Action
    view.rs         # View trait
    migrations/     # diesel SQLite schema
data/               # world definition (TOML) loaded at startup
examples/           # custom view + GUI-style front-end demos
src/                # the terminal front-end (TextView + CLI loop)
```

World content lives in TOML files: `data/items.toml` describes items
(`id`, `primary_name`, `aliases`), `data/rooms.toml` describes rooms and
which items they contain.

## Customizing

```rust
struct MyRules;
impl Rules for MyRules {
    fn on_take(&mut self, world: &mut WorldState, name: &str, resolution: Resolution) -> Vec<Event> {
        // veto, mutate, or hand off to BasicRules
    }
}

let engine = GameEngine::open_with_rules("typegin.db", &data, MyRules)?;
```

```rust
struct PirateView;
impl View for PirateView {
    fn render(&self, events: &[Event], _world: &WorldState) -> Vec<String> {
        // say it like a pirate
    }
}
```

## Tests

```sh
cargo test
```
