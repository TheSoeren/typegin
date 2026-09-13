# AGENTS.md

## Project

Text-adventure engine in Rust. Workspace with two engine crates plus a
workspace-level `examples/` directory holding the front-ends:

- `crates/core` — library (parsing, world state, rules, data-driven
  interactions, NPC dialogue). Everything gameplay-relevant lives here; it
  has no I/O, terminal, or GUI/rendering code of any kind — not even a
  `View` trait. It hands a front-end a typed `Event` stream and read-only
  `WorldState` queries and has no further opinion; rendering is entirely
  each front-end's own concern.
- `crates/render` — a reusable point-and-click **rendering engine** for
  `core`, built on `macroquad`. It is not a game and not a prototype of one:
  like `core` never embeds a specific game's rooms/items/puzzles, `render`
  never embeds a specific game's assets, hotspot coordinates, or content —
  it must stay reusable across multiple different PnC games, and carries no
  `src/main.rs` of its own. It reacts to `core`'s `Event` stream (animation,
  movement, ...) and is driven by whatever `WorldState`/`extra` data a
  consuming game supplies, plus an asset-provider extension point a
  specific game implements (design not finalized yet). Unlike `examples/text`,
  `render` doesn't need to serve all three front-end modalities (see
  "Non-negotiable" below) — it's free to build whatever a PnC UI
  specifically needs (hotspots, a radial verb coin, drag interactions)
  using `GameEngine::interactions_for`/`verbs_for`, without worrying about
  staying parser-compatible.
- `examples/` — a **workspace-level** directory (ordinary sibling crates
  grouped here, not cargo's per-crate `examples/` convention):
  - `examples/data/*.yaml` — the actual shipped game content (an *Edna &
    Harvey: The Breakout* homage), the single source of truth every
    front-end below reads. One world, one copy, so a change to it is
    visible from every consumer at once.
  - `examples/text` (binary `text-proto`) — the maintained terminal
    front-end (`src/main.rs`, `src/view.rs`). Reads `examples/data/*.yaml`,
    concatenates it into one YAML document, and hands that to
    `core::WorldData::from_yaml`. Matches `Event`s directly against
    hand-written prose (no shared rendering trait — core doesn't define
    one, and this is the only maintained text consumer, so there's nothing
    to share it with).
  - `examples/pnc` (binary `pnc-example`) — a minimal, throwaway macroquad
    binary exercising `crates/render` against the same shared content:
    boots a `GameEngine`, draws hotspots from each object's authored
    `extra.gui.hotspot`, shows a verb coin on hover, executes a verb on
    click. Plain rectangles and `Debug`-printed verb labels, not real art —
    proves the render engine's wiring, not what a shipped game should look
    like.

`core`'s public surface is one flat re-export list from `crates/core/src/lib.rs`
— when exploring, start there rather than guessing module paths.

## North star goal (important, drive all feature work toward this)

The engine's unifying aim is to cover **every engine-level feature a modern
narrative point-and-click adventure needs** — the genre as shipped today by
studios like Daedalic (_Deponia_, _The Whispered World_, _A New Beginning_)
and its contemporaries, not the much sparser command-parser adventures of the
1980s. When weighing a feature or design choice, ask: "does this move us
toward being able to ship a modern point-and-click title?" Any engine-level
gap against that bar is a defect against this goal — and the bar should be
read generously: prefer scoping in a genuinely load-bearing modern-genre
feature over deferring it for being unfamiliar.

_Deponia_ is the reference point for scope, not a spec to hardcode — see
"Non-negotiable" below. The mechanical adventure core (rooms, doors, items,
take/drop/examine/use, and the point-and-click `interactions_for` query)
works. Puzzle logic is data-driven interactions in YAML world data (verbs,
items, conditions, effects — `InteractionData` in
`crates/core/src/data/interactions_data.rs`, authored under
`data/interactions.yaml`) instead of Rust closures, though a consumer can
still supply closures via `Rules::interactions()` for anything data-driven
interactions don't cover. Content-layer status, in the order they were
tackled:

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
3. **Room-event / trigger system** — done. Non-item-triggered beats (entering
   a room, a flag transitioning), the analogue of a "World" hook distinct
   from `Interaction`'s verb-object shape. `TriggerData` (schema in
   `crates/core/src/data/trigger_data.rs`, authored under `data/triggers:`)
   reuses the existing `DataCondition`/`DataEffect` vocabulary; runtime
   dispatch lives in `crates/core/src/trigger.rs`
   (`check_triggers(&mut WorldState)`), called from the tail of
   `GameEngine::execute_action` so it runs after _every_ action, not just a
   specific verb. Dispatch contract: on each call, every not-yet-fired
   trigger's condition is checked against a readiness snapshot taken before
   any trigger in that pass runs its effects, so one trigger's effect cannot
   make another trigger in the same pass newly eligible (a chain resolves
   over multiple player turns, never within one); all triggers ready in a
   pass fire, in declaration order; each trigger fires at most once, ever
   (`WorldState`'s `fired_triggers` set, no re-arm). See
   `crates/core/tests/triggers.rs` (its module doc comment states the
   contract) and `crates/core/src/trigger.rs`'s module doc comment.
4. **Point-and-click verb-coin UI primitives** (`GameEngine::verbs_for`,
   `Rules::default_verbs`) and **combine-two-carried-items** (`use <item> on
<other carried item>`, `TargetKind::Carried`) — in progress. `verbs_for`
   answers "what verbs apply to this target" as a pure function of the
   target and world state alone (item-agnostic interactions unioned with
   `default_verbs`, deduplicated) — it deliberately does **not** vary by what
   the player happens to be carrying, matching how a point-and-click UI's
   verb coin and "reveal hotspots" affordance actually behave: static per
   room state, never inventory-reactive. Whether a specific carried item does
   something to a target is a _separate_ question, answered by
   `interactions_for(Some(item), Some(target))` at the moment that item is
   actually used against the target (a drag-and-drop, a click with an item
   selected) — never surfaced through the coin itself. See
   `crates/core/tests/verb_coin.rs` and `crates/core/tests/combine.rs`.
5. **Persistable world state (save/load)** — in progress.
   `GameEngine::save`/`GameEngine::load` are a _progress snapshot_, not a
   `WorldState` dump: `WorldState` also carries a full copy of the static
   authored content (`data_interactions`, `triggers`, `object_templates`,
   every NPC's dialogue tree), so serializing it wholesale would duplicate
   that content into every save and let an old save pin a stale copy of it
   past a content patch. `save` persists only the dynamic slice (flags,
   inventory, room object membership, objects discarded out of the world
   entirely, door lock state, fired triggers); `load(data, rules, save)`
   rebuilds fresh static content from `data` (which may differ from what
   `save` was taken against — a content patch between save and load must be
   picked up) and restores the dynamic slice on top. Deliberately excluded:
   in-progress dialogue (`dialogue_state`/`active_npc`) — a reload never
   resumes the player mid-conversation; every front-end modality dispatches
   dialogue as an immediate `Action` off the player's last input, none of
   them need `load` to land back inside a conversation turn. See
   `crates/core/tests/save_load.rs`.

Feature work should be judged against these; when a step maps to one of them,
say so explicitly when handing off a spec.

### Non-negotiable: it stays an ENGINE, and the consumer stays in control

The genre bar above is a _benchmark to test completeness_, never a spec to
hardcode. The engine must remain general-purpose so it can ship **any**
adventure game, not just something shaped like Deponia. In particular:

- **Never bake any specific game's content, characters, verbs, or puzzle
  logic into the core.** Named games are test cases / completeness metrics,
  not products to embed. A feature is only worth adding insofar as it
  generalizes to any adventure (dialogue trees, flags, triggers, verb-coin
  queries, ...).
- **The same rule extends to `crates/render`: it stays a reusable rendering
  engine, never a specific game.** No room/item/interaction data, no
  hardcoded object ids, no specific game's assets anywhere in its `src/`,
  and no `src/main.rs` at all — a library crate with a `main.rs` reads as
  "this crate is the game," which is exactly what it must not be. Anything
  demonstrating it against real or placeholder content belongs in the
  workspace-level `examples/` directory (`examples/pnc`), a separate crate
  outside `crates/render` entirely. A specific PnC game is likewise its own
  separate crate that supplies its own `WorldData`, assets, and
  hotspot/`extra` content, and depends on both `core` and `render`.
- **Preserve the consumer's freedom to pick between the three front-end
  modalities.** A consumer always chooses exactly _one_ modality for their
  actual game — this is not a claim that every game must ship as all three
  at once. The obligation is on the _engine_: it must never implement
  anything that would foreclose any of the three as a possible choice for
  some future consumer. The three:
  1. **text-in / text-out** (pure terminal parser + prose view),
  2. **text-in / GUI-out** (parser drives commands, GUI renders events/state),
  3. **full point-and-click** (no parser needed at all: a GUI synthesizes
     `Action`s from clicks, using `interactions_for`/`verbs_for`, and renders
     by reading `Event`s/`WorldState` directly — core has no rendering
     contract of its own for it to render _via_).
     No design choice may assume one modality or silently close the door on
     another — that includes convenience queries like the verb-coin primitives
     above: `verbs_for` exists _in addition to_ the parser path, an option a
     point-and-click consumer can lean on, never a replacement the other two
     modalities are forced through. Anything that would force a specific input
     source or output rendering is a defect. Guard this when adding verbs, the
     `interactions_for`/`verbs_for` queries, and `Event`/`Rules` shapes — and
     never add a rendering-facing type (a `View` trait, a render-command enum,
     ...) back into core; that's each front-end's own crate's job now
     (`examples/text`, `crates/render`).
- **Core parses exactly one YAML document.** `WorldData::from_yaml`/`load`
  take a single string/path, not a fixed set of named files — how a
  consumer organizes authored content across files (one file, five files,
  one per room) is entirely their call. `examples/text` happens to read
  the conventional `data/{globals,items,rooms,interactions,npcs}.yaml` split
  and concatenates them before calling `from_yaml`, but that convention lives
  in that crate, not in core.

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
  the TDD split above governs _new feature_ work, not explicitly-scoped fixes/refactors. Still don't wander beyond the scope given.
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

| Task                  | Command                                                                                                                                                                                                                                                            |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Build                 | `cargo build`                                                                                                                                                                                                                                                      |
| Run (text front-end)  | `cargo run -p text-proto`                                                                                                                                                                                                                                          |
| Run (PnC example)     | `cargo run -p pnc-example`                                                                                                                                                                                                                                         |
| Test (all)            | `cargo test`                                                                                                                                                                                                                                                       |
| Test (unit tier only) | `cargo test -p core --lib`                                                                                                                                                                                                                                         |
| Test (single suite)   | `cargo test --test <name>` (names: `combine`, `data_interactions`, `default_rules`, `doors`, `drop`, `extras`, `flags`, `hidden`, `input`, `interactions`, `navigation`, `npcs`, `rules_override`, `save_load`, `symbolic_keys`, `triggers`, `verb_coin`, `world`) |
| Lint                  | `cargo clippy --workspace --all-targets`                                                                                                                                                                                                                           |
| Format                | `cargo fmt`                                                                                                                                                                                                                                                        |
| Format check          | `cargo fmt --check`                                                                                                                                                                                                                                                |

Recommended verification order: `cargo fmt --check && cargo clippy --workspace --all-targets && cargo test`

`clippy::pedantic` is `deny`-level workspace-wide (plus explicit denies for
`redundant_clone`, `clone_on_copy`, `needless_collect` — see the workspace
`Cargo.toml`'s `[workspace.lints.clippy]`), so most borrowing/cloning/API-shape
idioms are already enforced at build time, not just style preference.
