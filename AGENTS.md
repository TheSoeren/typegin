# AGENTS.md

## Project

Text-adventure engine in Rust. Workspace with two crates:

- `crates/core` — library (parsing, world state, rules, view trait)
- Root `typegin` crate — terminal front-end (`src/main.rs`, `src/view.rs`)

## North star goal (important, drive all feature work toward this)

The engine's unifying aim is to cover **every engine feature needed to
theoretically build [Edna & Harvey: The Breakout](https://en.wikipedia.org/wiki/Edna_%26_Harvey%3A_The_Breakout)**
(a Daedalic point-and-click adventure) — so that any engine-level gap is a
defect against this goal. When weighing a feature or design choice, ask:
"does this move us toward being able to ship an Edna & Harvey title?"

The mechanical adventure core (rooms, doors, items, examine/use/combine, and
the point-and-click `interactions_for` query) already works. The **content
layer** that such a game needs is the priority gap — in rough order of leverage:

1. **Data-driven game logic** — authored interactions/conditions/effects
   should live in YAML world data, not Rust closures (content the designer can
   author without writing Rust).
2. **Flags / global quest / causal state** — a first-class `flags`-style model
   on `WorldState` so "when X happens then Y" and conditional states are
   declarative, not bolted on via closures.
3. **NPCs and dialogue trees** — a character/dialogue-graph model over flags.
4. **Room-event / trigger system** — non-item-triggered beats (entering a room,
   time/causal chains), the analogue of a "World" hook distinct from
   `Interaction`'s verb-object shape.
5. **Inventory / verb-coin UI primitives** and **combine-two-carried-items**
   scope (a distinct Take-vs-combine overlap).

Feature work should be judged against these; when a step maps to one of them,
say so explicitly when handing off a spec.

### Non-negotiable: it stays an ENGINE, and the consumer stays in control

Edna & Harvey is a *benchmark to test completeness*, never a spec to hardcode.
The engine must remain general-purpose so it can ship **other, different
adventure games** too. In particular:

- **Never bake Edna & Harvey's specific content, characters, verbs, or puzzle
  logic into the core.** It's a test case / completeness metric, not a product
  to embed. Its features are only worth adding insofar as they generalize to
  any adventure (dialogue trees, flags, triggers, etc.).
- **Preserve the consumer's freedom to pick between the three front-end
  modalities** — meaning a single game can be played three ways:
  1. **text-in / text-out** (pure terminal parser + prose view),
  2. **text-in / GUI-out** (parser drives commands, GUI renders events/state),
  3. **full point-and-click** (no parser needed at all: a GUI synthesizes
     `Action`s from clicks and uses `interactions_for`, rendering via
     `RenderCommand` or reading `Event`s/`WorldState` directly).
- The engine must keep these three interchangeable — **no design choice may
  assume one modality**, or silently close the door on another. Anything that
  would force a specific input source or output rendering is a defect. Guard
  this when adding verbs, the `interactions_for` query, `Event`/`RenderCommand`
  shapes, and the `View`/`Rules` traits.

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
- World content is YAML: `data/items.yaml` and `data/rooms.yaml`. Test fixtures live in `crates/core/data/`.
- `GameEngine` is a pure in-memory `WorldState` built from world data (YAML); there is no database.
- All tests are integration tests under `crates/core/tests/` with shared helpers in `tests/common/mod.rs`.
- `GameEngine::get` uses `BasicRules` (stock defaults). `get_with_rules` injects custom `Rules`. The stock `BasicRules` already implements take/drop, refuses to take scene objects (`CantTake`) and unlocks doors whose `gated_by` object is used on them; the terminal front-end in `src/main.rs` only adds a `GameRules::on_look` that reveals the hidden passage door.
- `View` is a render trait producing a `RenderCommand` stream: the default `render(&mut self, events, world)` dispatches each event to a typed `render_*` hook (defaults silent). It observes `Event`s + `&WorldState` and can never mutate the game.
- NEVER use `deref`
- NEVER use `unwrap`

## Architecture

`text input → tokenizer → lexer → Action → GameEngine.handle_input → Rules hook → Events + mutated WorldState → View.render (RenderCommand stream) → front-end interpreter`

Public API surface is re-exported from `crates/core/src/lib.rs`. The `input` module is private to the crate.

## Backlog

- **Opaque game event passthrough** — add something like `Event::Custom { name: String }` so custom `Rules` can emit game-specific beats and consumer `View`s render them; the last big engine reusability unlock.
- **Game-side proof of doors** — front-end `Rules` making "use iron key on door" read the exit `extra` (`opens_with`), call `unlock_exit`, and reveal a hidden door, proving the lock/door/extra loop end-to-end from the consumer side.
