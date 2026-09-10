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

The mechanical adventure core (rooms, doors, items, take/drop/examine/use, and
the point-and-click `interactions_for` query) works, and **puzzle logic is now
written as data-driven interactions** in YAML world data (verbs, items,
conditions, effects — see `data/interactions.yaml`) instead of Rust closures.
The **content layer** gaps that remain, in rough order of leverage:

1. **Flags / global quest / causal state** — a first-class `flags`-style model
   on `WorldState` so "when X happens then Y" and conditional states are
   declarative, not bolted on via closures (`player_holds`/`exit_*` data
   conditions are the current stand-ins).
2. **NPCs and dialogue trees** — a character/dialogue-graph model over flags.
3. **Room-event / trigger system** — non-item-triggered beats (entering a
   room, time/causal chains), the analogue of a "World" hook distinct from
   `Interaction`'s verb-object shape.
4. **Inventory / verb-coin UI primitives** and **combine-two-carried-items**
   scope (a distinct Take-vs-combine overlap).

Feature work should be judged against these; when a step maps to one of them,
say so explicitly when handing off a spec.

### Non-negotiable: it stays an ENGINE, and the consumer stays in control

Edna & Harvey is a _benchmark to test completeness_, never a spec to hardcode.
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

| Task                | Command                                                                                                                                                                                          |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Build               | `cargo build`                                                                                                                                                                                    |
| Run                 | `cargo run`                                                                                                                                                                                      |
| Test (all)          | `cargo test`                                                                                                                                                                                     |
| Test (single suite) | `cargo test --test <name>` (names: `input`, `world`, `navigation`, `drop`, `default_rules`, `rules_override`, `hidden`, `extras`, `doors`, `interactions`, `data_interactions`, `symbolic_keys`) |
| Lint                | `cargo clippy --workspace --all-targets`                                                                                                                                                         |
| Format              | `cargo fmt`                                                                                                                                                                                      |
| Format check        | `cargo fmt --check`                                                                                                                                                                              |

Recommended verification order: `cargo fmt --check && cargo clippy --workspace --all-targets && cargo test`
