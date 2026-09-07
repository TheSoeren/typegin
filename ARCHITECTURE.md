# Architecture: a unified object model, Visionaire-style, with a data-driven interaction layer

This document describes the engine as it is today: what the model looks like,
why, and how to build on it.

## Why

The engine is meant to power three kinds of front-ends from one core:

1. **Strict text adventure** — the player types commands.
2. **Text input, GUI output** — the player still types, but the world is
   rendered graphically.
3. **Point-and-click adventure** — the player clicks; the GUI must know *what
   can be done with what* and translates the click into a game action.

We researched how classic engines solve this. Inform 7 / TADS unify *every*
interactable into one "thing" and resolve nouns across a scope. Visionaire
(and AGS) go further: **there are exactly two object kinds — scene objects and
inventory objects**; verbs are *data*, interactions are authored per object,
everything has a fallback answer, and the GUI can enumerate interactions. This
engine follows the Visionaire path, because it maps most directly onto
requirement #3 — and adds a *data-driven* interaction layer so the authoring
surface is YAML, not Rust.

## The model

### 1. One interactable: an `Object`, with exactly two kinds

Every interactable is an `Object` keyed by a **symbolic string** (`ObjectId`,
e.g. `chair-leg`), with names and opaque `extra`. An object has one of two
kinds:

- `Item` — an inventory object: portable, taken into inventory (a key, a
  sword). This is the default kind when world data omits it.
- `Scene` — a scene object: stays in the world, clickable/examinable but never
  portable (furniture, fixtures… and every door).

A **door is just a Scene object carrying optional door data**:

```yaml
- key: cell-door
  primary_name: Cell door
  aliases: [cell door, door]
  kind: Scene
  door:
    direction: east
    to: corridor
    locked: true
```

`resolve_target(name)` resolves one noun path against visible room objects and
carried objects, and returns `Found(ObjectId)` / `Ambiguous { ids }` /
`NotFound`. Declaring `door` data forces the kind to `Scene`, keeping the
two-kind invariant at runtime.

### 2. Rooms hold objects; the direction index is derived

`Room` holds `objects`, `hidden_objects` and a *derived*
`HashMap<Direction, ObjectId>` built once from the door objects in both lists —
a cache for O(1) movement, not an identity store. "Hidden" is *list
membership*, not a flag on the door: a hidden door is an object sitting in
`hidden_objects` until revealed. The `locked` flag stays on the door data
(`gated_by` optionally links the door to its unlocking object as a pure fact).

Door helpers over the index: `get_room_id_by_exit_direction`,
`is_exit_locked` / `is_exit_hidden`, `exit_directions` (open ones only),
`exit_info` (the door's `ObjectInfo`), `exit_extra`, `exit_gated_by`,
`unlock_exit` / `lock_exit`, `reveal_exit` / `hide_exit`. `reveal_object` and
`reveal_exit` are the same operation but on different planes (pluck a hidden
object vs. move a door between lists).

### 3. The scene-vs-inventory distinction is stock behaviour

`BasicRules::on_take` is where the kind distinction bites:

- `Item` in the room → taken into inventory (`Took`),
- `Scene` (including every door) → refused with `CantTake { object }`,
- missing/ambiguous → the usual not-found/ambiguous events.

Because stock behaviour covers take/drop/examine/use and the data layer covers
the puzzles, the terminal front-end runs the stock `GameEngine::get` with no
custom `Rules` implementation at all.

### 4. Data-driven interactions: the authoring surface

The engine's flagship authoring surface lives in `data/interactions.yaml`, not
Rust closures. Shape:

```yaml
- verb: use
  item: chair-leg
  target:
    object: table
  effect:
    - discard: chair-leg
    - grant: broken-chair-leg
    - emit: You lay the chair leg across the table...
```

- **`verb`** — one of `look`, `go`, `examine`, `take`, `drop`, `use`.
- **`item`** — the object the player must be using/carrying; absent matches
  any.
- **`target`** — omitted matches any target *including self-use*; `object: <id>`
  matches one exact target; `kind: scene` matches any scene target. The coarse
  `kind` filter is complemented by door-specific conditions.
- **`condition[]`** — all must hold (AND); gates both dispatch and the
  `interactions_for` query. Catalog: `room`, `player_holds`, `exit_locked`,
  `exit_hidden`, `is_door`, `not` (negation of any condition).
- **`effect[]`** — run in order, each mutating the world and optionally
  emitting one event:
  - `emit` → `Event::Custom { name }` (game-authored prose beat),
  - `take` → room → inventory (`Took`), `drop` → inventory → room (`Dropped`),
  - `grant` → materialise into inventory **from the object-template registry,
    even if the object sits in no room** (`Granted`),
  - `discard` → remove a carried item without placing it in the room
    (`Discarded`),
  - `unlock_exit` / `lock_exit`, `reveal_exit` / `hide_exit`,
    `reveal_object` / `hide_object` (mostly silent).

Validation is done at load time (`validate_references`) so a typo'd object key
or duplicate key fails the build, not the playtest.

**Dispatch precedence** in every verb hook: first matching data interaction
(declaration order, first wins) → then consumer closure interactions
`Rules::interactions()` → then the stock fallback spine. A puzzle authors one
interaction ("use the toenail on the vent grille") and the engine answers for
everything else with refusals (`UsedTargetNeeded`, `CannotUse`, not-found/
ambiguous) without the author re-implementing them. Interactions can be gated
on world state (`exit_hidden`, `player_holds`, ...) so a beat fires exactly
once (e.g. "examine the chair to reveal the leg" only while the leg is still
in the room).

### 5. The point-and-click hook: `GameEngine::interactions_for`

A GUI asks "what can the player do with this target right now?":

```rust
engine.interactions_for(Some(toenail_id), Some(vent_grille_id))
```

The query runs the *same* `matches()` logic (verb + item + target filter +
conditions) used by dispatch, so it only reports interactions that are
currently valid. Data interactions are compiled once at construction into the
closure `Interaction` shape, so the query answers data *and* closure
interactions uniformly — no duplicate logic for the menu vs. the execution.

### 6. Events: typed, object-flavoured and open

All payloads carry object ids/names. The movement/take/drop/use/examine
families each have their not-found and ambiguous variants. Recent additions:

- `Event::Custom { name }` — an opaque, game-authored beat (the prose is the
  responsibility of the `View`).
- `Event::UnlockedExit { direction }`, `CannotUse { item, target }`,
  `CantTake { object }`.
- `Event::Granted { object_id, object }` / `Event::Discarded { object_id, object }`
  — inventory changes that do not come from a room.

### 7. Rendering: one pipeline, many front-ends

Text and GUI front-ends share a single render pipeline:
`View::render(&mut self, events, world) -> Vec<RenderCommand>`.

- The default `render` dispatches each `Event` to a **typed, per-event hook**
  (`render_took`, `render_went_exit_locked`, ...) with the payload already
  destructured. A view overrides only the events it phrases; every hook
  defaults to silence, so a new engine event never breaks existing views.
  Unknown events fall back to `render_generic`.
- Output is a stream of `RenderCommand` (`Line`, `ClearScreen`, ...) — a
  `#[non_exhaustive]` enum the engine can extend.
- `render` takes `&mut self` so stateful views can pace output or animate; the
  view can read *only* `WorldState` — it can never mutate the game.

## The pipeline today

```
text input → tokenizer → lexer → Action
  → GameEngine.handle_input → execute_action
      → resolve_target / resolve_player_object / resolve_room_object
      → Rules hook
          → data interaction? (conditions re-checked; first match wins)
          → else closure Interaction? (Rules::interactions())
          → else stock fallback (kind check, gated unlock, refusals)
      → Vec<Event> + &mut WorldState
  → View.render → Vec<RenderCommand> (Line, ClearScreen, ...)
      → front-end interpreter (print / widget tree)
```

A point-and-click front-end is just a View plus the `interactions_for` query:
it synthesizes an `Action` from clicks, sends it through the exact same
`execute_action` path, and renders `Event`s.

The terminal front-end currently renders, on `look`, the room's description
(from the room `extra`), the visible objects, the carried inventory, and a
survey of the compass directions that have exits (hidden doors stay hidden;
locked doors are marked). All puzzle logic lives in `data/interactions.yaml`.

## Code map

| File / module | Role |
| --- | --- |
| `crates/core/src/world/object.rs` | `ObjectId` (symbolic string key), `ObjectResolution`, `Object` (kind + door state), `ObjectInfo`/`DoorInfo` |
| `crates/core/src/world/room.rs` | `Room`: visible/hidden object lists, derived `Direction → ObjectId` index, door helpers |
| `crates/core/src/world/mod.rs` | `WorldState`: unified `resolve_target`, scope helpers, `object_kind`/`object_is_door`/`exit_direction_of`, transfers, `player_grant_object`/`player_discard_object`, object-template registry, `from_data` |
| `crates/core/src/interaction.rs` | `Verb`, `ActionContext`, `TargetFilter` (Any/Targeted/Scene), closure `Interaction` |
| `crates/core/src/data/mod.rs` | `WorldData`, `ExtraValue`, `load`/`from_yaml` + reference validation |
| `crates/core/src/data/object_data.rs` | `ObjectData` (`key`, `primary_name`, `aliases`, `kind`, `door`, `extra`) |
| `crates/core/src/data/room_data.rs` / `door_data.rs` | `RoomData`, `DoorData` (direction/to/locked/gated_by as plain strings) |
| `crates/core/src/data/interactions_data.rs` | `InteractionData`, `DataTarget`, `DataCondition`, `DataEffect`, `validate_references`, `compile` → `Interaction`, `dispatch_data` |
| `crates/core/src/input/` | tokenizer, lexer, `Direction` (compass, `Display`), `Action` + result types (`TakeResult`, `DropResult`, `GrantResult`, `DiscardResult`, ...); private module, re-exported |
| `crates/core/src/rules.rs` | `BasicRules`, `Rules` (defaults): stock take/drop/examine/use with data+closure interaction dispatch, gated unlock, refusals |
| `crates/core/src/engine.rs` | Action → hook dispatch, `handle_input`, `interactions_for` query |
| `crates/core/src/event.rs` | `Event` enum (typed variants incl. `Custom`, `UnlockedExit`, `CannotUse`, `CantTake`, `Granted`, `Discarded`) |
| `crates/core/src/view.rs` | `View` trait: `render` dispatches to typed `render_*` hooks (defaults silent, `render_generic` fallback), `RenderCommand` (`#[non_exhaustive]`) |
| `src/main.rs`, `src/view.rs` | terminal front-end: loads the three YAML files, stock `GameEngine::get`, REPL; `TextView` phrases events (description + objects + inventory + exits on `look`) |

Public API surface is re-exported from `crates/core/src/lib.rs`.

## Data mapping

Three YAML files, one per concept:

- **`data/items.yaml`** — `objects:` keyed by symbolic `key`; `kind: Item |
  Scene` (default `Item`), optional `door:` block, opaque `extra` (any YAML
  value → `ExtraValue`).
- **`data/rooms.yaml`** — `rooms:` keyed by `key`; `visible_objects` /
  `hidden_objects` lists of object keys, `extra` (the terminal front-end reads
  `extra.description`).
- **`data/interactions.yaml`** — `interactions:` list of `verb` + optional
  `item`/`target`/`condition` + `effect` (see section 4).

`WorldData::load` / `from_yaml(items, rooms, interactions)` parse and validate;
a reference to an unknown key anywhere fails loudly.

## One important design note

Authored interactions and defaults must live on the **same concrete `Rules`
type**. `BasicRules::on_use` consults both the data interactions and
`interactions()` so a consumer type that provides interactions gets the stock
fallback *plus* its additions for free — but only when that type is what the
engine dispatches on. A wrapper that delegates `on_use` to a plain `BasicRules`
silently loses the wrapper's interactions. Provide `interactions()` on your own
`Rules` impl instead of nesting.

## Follow-ups

- Data-driven interactions (see AGENTS north-star #1) are done: the game's
  entire escape sequence is authored in YAML with the stock engine.
- Natural next steps — flags/global quest state, NPCs + dialogue trees,
  room-event/trigger beats, and inventory/verb-coin UI primitives; see the
  priority order in `AGENTS.md`.