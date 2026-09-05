# AGENTS.md

## Project

Text-adventure engine in Rust. Workspace with two crates:

- `crates/core` — library (parsing, world state, rules, view trait)
- Root `typegin` crate — terminal front-end (`src/main.rs`, `src/view.rs`)

## Working style (important)

- **TDD split of labour:** I implement the feature; YOU write the test cases. Do NOT write feature/implementation code unless I explicitly
  ask you to. If a task needs feature logic written, write the (failing) test suite first and hand it to me as the spec, then let me make
  it green.
- **The failing, "red-phase" test is the deliverable** for a new step. Run it and confirm it fails for the right reason before reporting it
  to me, so I know exactly what to implement.
- **When I ask a design/API question** (struct shape, method signatures, `Entry` vs owned values, etc.), answer the question first with a
  clear explanation and options — do not jump into editing code until I confirm which approach I want.
- **I sometimes delegate specific fixes directly** (e.g. "fix my `hidden_exit_directions`", "fix `remove_exit`", "re-export this type").
  In those cases it is fine to edit the code I point at; do not touch related feature logic beyond the scope I gave.
- **When the right next step or approach is ambiguous, ask me** (use the `question` tool). I prefer confirming direction over guessing.
- After implementing/updating, run the verification order (below) and report pass/fail concisely so I can react.

## Commands

| Task                | Command                                                                                                                 |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Build               | `cargo build`                                                                                                           |
| Run                 | `cargo run`                                                                                                             |
| Test (all)          | `cargo test`                                                                                                            |
| Test (single suite) | `cargo test --test <name>` (names: `input`, `world`, `navigation`, `drop`, `default_rules`, `rules_override`, `hidden`, `extras`, `doors`) |
| Lint                | `cargo clippy --workspace --all-targets`                                                                                |
| Format              | `cargo fmt`                                                                                                             |
| Format check        | `cargo fmt --check`                                                                                                     |

Recommended verification order: `cargo fmt --check && cargo clippy --workspace --all-targets && cargo test`

## Key conventions

- Rust edition 2024. No rustfmt.toml or clippy.toml; defaults apply.
- World content is TOML: `data/items.toml` and `data/rooms.toml`. Test fixtures live in `crates/core/data/`.
- `GameEngine` is a pure in-memory `WorldState` built from world data (TOML); there is no database.
- All tests are integration tests under `crates/core/tests/` with shared helpers in `tests/common/mod.rs`.
- `GameEngine::get` uses `BasicRules` (stock defaults). `get_with_rules` injects custom `Rules`. The terminal front-end in `src/main.rs` provides `TakeRules` that overrides `on_take` — the core's `BasicRules` does **not** move items into inventory.
- `View` is a read-only render trait observing `Event`s + `&WorldState`. It can never mutate the game.
- NEVER use `deref`
- NEVER use `unwrap`

## Architecture

`text input → tokenizer → lexer → Action → GameEngine.handle_input → Rules hook → Events + mutated WorldState → View.render → text output`

Public API surface is re-exported from `crates/core/src/lib.rs`. The `input` module is private to the crate.

## Backlog

- **Opaque game event passthrough** — add something like `Event::Custom { name: String }` so custom `Rules` can emit game-specific beats and consumer `View`s render them; the last big engine reusability unlock.
- **Game-side proof of doors** — front-end `Rules` making "use iron key on door" read the exit `extra` (`opens_with`), call `unlock_exit`, and reveal a hidden door, proving the lock/door/extra loop end-to-end from the consumer side.
