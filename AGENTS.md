# AGENTS.md

## Project

Text-adventure engine in Rust. Workspace with two crates:

- `crates/core` — library (parsing, world state, rules, data-driven
  interactions, NPC dialogue, view trait). Everything gameplay-relevant lives
  here; it has no I/O, terminal, or GUI code.
- `crates/cli` (binary name `typegin`) — terminal front-end
  (`src/main.rs`, `src/view.rs`). Reads the shipped `data/*.yaml` files,
  concatenates them into one YAML document, and hands that to
  `core::WorldData::from_yaml`.

`core`'s public surface is one flat re-export list from `crates/core/src/lib.rs`
— when exploring, start there rather than guessing module paths.

## North star goal (important, drive all feature work toward this)

The engine's unifying aim is to cover **every engine feature needed to
theoretically build [Edna & Harvey: The Breakout](https://en.wikipedia.org/wiki/Edna_%26_Harvey%3A_The_Breakout)**
(a Daedalic point-and-click adventure) — so that any engine-level gap is a
defect against this goal. When weighing a feature or design choice, ask:
"does this move us toward being able to ship an Edna & Harvey title?"

The mechanical adventure core (rooms, doors, items, take/drop/examine/use, and
the point-and-click `interactions_for` query) works. Puzzle logic is
data-driven interactions in YAML world data (verbs, items, conditions,
effects — `InteractionData` in `crates/core/src/data/interactions_data.rs`,
authored under `data/interactions.yaml`) instead of Rust closures, though a
consumer can still supply closures via `Rules::interactions()` for anything
data-driven interactions don't cover. Content-layer status, in the order they
were tackled:

1. **Flags / global quest / causal state** — done. A first-class `flags`
   model on `WorldState` (`has_flag`/`set_flag`/`clear_flag`), with
   `DataCondition::Flag`/`DataEffect::SetFlag`/`ClearFlag` in the interaction
   schema. See `crates/core/tests/flags.rs`.
2. **NPCs and dialogue trees** — done. NPCs carry an inline dialogue graph
   (`DialogueNode`/`DialogueChoice`, branching via `choose`), and
   `examine`/`talk`/`use` are target-oriented (`WorldState::resolve_target`
   resolves a name to `Target::Npc` or `Target::Object`), so `use <item> on
   <npc>` works the same way as `use <item> on <object>`. NPCs are still
   room-bound, not carryable (no inventory-companion NPC). See
   `crates/core/tests/npcs.rs`.
3. **Room-event / trigger system** — spec written, implementation pending.
   Non-item-triggered beats (entering a room, a flag transitioning), the
   analogue of a "World" hook distinct from `Interaction`'s verb-object
   shape. The dispatch contract (a `TriggerData` list reusing the existing
   `DataCondition`/`DataEffect` vocabulary, checked after every action,
   one-shot, all-ready-fire-in-declaration-order, no same-turn cascade) is
   pinned down as a red-phase suite in `crates/core/tests/triggers.rs` —
   read its module doc comment before touching this area.
4. **Inventory / verb-coin UI primitives** and **combine-two-carried-items**
   scope (a distinct Take-vs-combine overlap) — not started.

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
- **Core parses exactly one YAML document.** `WorldData::from_yaml`/`load`
  take a single string/path, not a fixed set of named files — how a
  consumer organizes authored content across files (one file, five files,
  one per room) is entirely their call. `crates/cli` happens to read the
  conventional `data/{globals,items,rooms,interactions,npcs}.yaml` split and
  concatenates them before calling `from_yaml`, but that convention lives in
  the CLI, not in core.

## Working style (important)

- **TDD split of labour:** I implement the feature; YOU write the test cases. Do NOT write feature/implementation code unless I explicitly
  ask you to. If a task needs feature logic written, write the (failing) test suite first and hand it to me as the spec, then let me make
  it green.
- **The failing, "red-phase" test is the deliverable** for a new step. Run it and confirm it fails for the right reason before reporting it
  to me, so I know exactly what to implement.
- **When I ask a design/API question** (struct shape, method signatures, `Entry` vs owned values, etc.), answer the question first with a
  clear explanation and options — do not jump into editing code until I confirm which approach I want.
- **I sometimes delegate specific fixes, cleanups, or codebase-wide passes directly** (e.g. "fix my `hidden_exit_directions`", "implement
  these 10 findings", "optimize the codebase for idiom X"). In those cases it is fine to edit `src/` directly, including production logic —
  the TDD split above governs *new feature* work, not explicitly-scoped fixes/refactors. Still don't wander beyond the scope given.
- **I work in this same tree concurrently.** Before editing a file you haven't touched yet this turn, assume it may have changed — re-read
  it fresh rather than trusting an earlier read in the conversation, especially anything under active feature work (check `git status`/`git
  diff` if unsure what's mid-flight). Never touch `crates/core/tests/triggers.rs` or its fixtures without checking whether it's currently a
  deliberate red-phase spec I'm implementing against.
- **When the right next step or approach is ambiguous, ask me** (use the `question` tool). I prefer confirming direction over guessing.
- After implementing/updating, run the verification order (below) and report pass/fail concisely so I can react.

## Testing conventions

- **Two tiers.** `#[cfg(test)] mod tests` co-located in the `src/` file being
  tested cover pure logic in isolation (parsing, `Interaction`/`DataCondition`
  matching, `Room`/`Object`/`Player` mutation methods) — run these alone with
  `cargo test -p core --lib`. `crates/core/tests/*.rs` integration suites
  drive behavior through `GameEngine::handle_input`/`interactions_for` —
  that's the TDD red-phase surface for new features. Prefer adding to the
  tier that already covers the area; don't duplicate one tier's case in the
  other.
- **Fixtures are single-document YAML, minimally split.** Since
  `WorldData::from_yaml` takes one string, test fixtures follow "one shared
  base-world file per logical world, no separate file per scenario":
  `crates/core/tests/fixtures/{single_room_world,multi_room_world,keyed_world}.yaml`
  hold the shared rooms/items (each reused by every test file that needs that
  world — don't fork a copy per test file). Per-test interaction/NPC/trigger
  snippets are inline Rust raw-string literals in the test file that uses
  them (a local `const` if reused by 2+ tests in that file, otherwise inline
  at the call site) — not separate fixture files. `common::merge_yaml(&[...])`
  concatenates sections (each already contributes a disjoint top-level key,
  so concatenation is a lossless merge, no deep-merge logic needed).

## Commands

| Task                | Command                                                   |
| -------------------- | ---------------------------------------------------------- |
| Build                | `cargo build`                                             |
| Run                  | `cargo run`                                                |
| Test (all)           | `cargo test`                                              |
| Test (unit tier only)| `cargo test -p core --lib`                                |
| Test (single suite)  | `cargo test --test <name>` (names: `data_interactions`, `default_rules`, `doors`, `drop`, `extras`, `flags`, `hidden`, `input`, `interactions`, `navigation`, `npcs`, `rules_override`, `symbolic_keys`, `triggers`, `world`) |
| Lint                 | `cargo clippy --workspace --all-targets`                  |
| Format               | `cargo fmt`                                               |
| Format check         | `cargo fmt --check`                                       |

Recommended verification order: `cargo fmt --check && cargo clippy --workspace --all-targets && cargo test`

`clippy::pedantic` is `deny`-level workspace-wide (plus explicit denies for
`redundant_clone`, `clone_on_copy`, `needless_collect` — see the workspace
`Cargo.toml`'s `[workspace.lints.clippy]`), so most borrowing/cloning/API-shape
idioms are already enforced at build time, not just style preference.
