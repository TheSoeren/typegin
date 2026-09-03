# typegin

A small text-adventure engine in Rust. The core is a pure game state that
turns typed commands into structured `Event`s; any front-end (terminal, GUI,
web, ...) can render those events however it likes.

## Concepts

The engine is **MVC with a passive view**, wired up with two extension seams:

| Piece      | File                                         | Role                                        |
| ---------- | -------------------------------------------- | ------------------------------------------- |
| Model      | [`WorldState`](crates/core/src/world/mod.rs) | Rooms, items, inventory, player position    |
| Controller | [`GameEngine`](crates/core/src/engine.rs)    | Parses input -> `Action`, mutates the model |
| Output     | [`Event`](crates/core/src/event.rs)          | Typed, lossless result of an action         |
| View       | [`View`](crates/core/src/view.rs)            | Trait that renders events + world to text   |

- **`Rules`** (`engine.rs`) is the _inbound_ hook. Inject it via
  `GameEngine::get_with_rules` to override behaviour - examine an item
  differently, veto a `take`, run custom logic. It is a Strategy plugged into
  the controller. Every hook has a sensible default (the same behaviour
  `BasicRules` exposes), so a custom type only implements the hooks it wants to
  change.
- **`View`** is the _outbound_ hook. It only ever observes `Event`s and a shared
  `&WorldState` - it can never mutate the game. The trait lives in
  `crates/core/src/view.rs`; the terminal front-end ships a `TextView`
  (`src/view.rs`) with the default player-facing wording. Swap in your own
  `View` for different prose or map `Event`s straight to a GUI.

## Quick start

```sh
cargo run
```

Type `look` to see the room, `quit` to leave.

### Commands

| Command                  | Shorthand | Meaning                        |
| ------------------------ | --------- | ------------------------------ |
| `look`                   | `l`       | Describe the current room      |
| `go north`               | `n`       | Move in a direction (`e/s/w`)  |
| `examine <thing>`        | `x`       | Inspect an item                |
| `take <item>`            | `get`     | Pick up an item into inventory |
| `drop <item>`            | `d`       | Put an item back in the room   |
| `use <item>`             |           | Use an item                    |
| `use <item> on <target>` |           | Use an item on a target        |
| `quit` / `exit`          |           | Leave the game                 |

## Layout

```
crates/core/        # engine library: state, input parsing, rules, view
  src/
    engine.rs       # GameEngine + Rules trait (defaults) + BasicRules
    world/          # WorldState, items, rooms, player
    event.rs        # Event enum
    input/          # tokenizer + lexer: text -> Action
    data.rs         # world data (TOML) types + loading
    view.rs         # View trait
  data/             # world fixtures + test worlds (TOML)
  tests/            # integration tests (see Tests below)
data/               # default runtime world definition (TOML)
src/                # the terminal front-end (TextView + CLI loop)
```

The engine world content lives in TOML: `data/items.toml` describes items
(`id`, `primary_name`, `aliases`), `data/rooms.toml` describes rooms, the items
they contain, and optional exits to other rooms. Point the binary at a different
data folder via its first argument (defaults to `data/`).

## Customizing

Override just the hooks you care about; the rest fall back to the defaults.
`BasicRules` is an empty `Rules` impl that exists purely as a name for "use the
defaults". Pass `BasicRules` (or `GameEngine::open`) for the stock behaviour.

```rust
struct MyRules;

impl Rules for MyRules {
    // Only on_take is customized; every other hook keeps its default.
    fn on_take(&mut self, world: &mut WorldState, name: &str, resolution: Resolution) -> Vec<Event> {
        match resolution {
            Resolution::Found(id) => {
                world.move_item_to_inventory(id);
                vec![Event::Took { item: name.to_string() }]
            }
            _ => vec![Event::Message("You can't do that.".to_string())],
        }
    }
}

let engine = GameEngine::get_with_rules(&data, MyRules);
// or use the defaults directly:
let engine = GameEngine::get(&data);
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

All tests are integration tests under `crates/core/tests/`, sharing helpers in
`tests/common/`:

| File                | Covers                                              |
| ------------------- | --------------------------------------------------- |
| `input.rs`          | Tokenizing + lexing + parsing text into `Action`s   |
| `world.rs`          | World data, entity resolution, item movement        |
| `navigation.rs`     | Room exits, movement, engine `on_go`                |
| `drop.rs`           | `drop` lexing, world drop, engine `on_drop`, flows  |
| `default_rules.rs`  | The stock behaviour of every `Rules` default        |
| `rules_override.rs` | Custom `Rules` overrides actually win over defaults |

Run a single suite with, e.g. `cargo test --test navigation`.
