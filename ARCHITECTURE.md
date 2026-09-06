# Architecture: a unified object model, Visionaire-style

This document explains the engine architecture after the rewrite: what changed,
why, and how to build on it.

## Why

The engine is meant to power three kinds of front-ends from one core:

1. **Strict text adventure** — the player types commands.
2. **Text input, GUI output** — the player still types, but the world is
   rendered graphically.
3. **Point-and-click adventure** — the player clicks; the GUI must know *what
   can be done with what* and translates the click into a game action.

The old core couldn't satisfy #2 and #3:

- **Items and exits lived on different planes.** `resolve_any_item` only
  resolved things that could be carried; a door could be *passed through* but
  never *addressed*. There was no way to express "use the iron key on the oak
  door" as a noun/verb problem, and no way for a GUI to learn "this door can be
  unlocked".
- **Behaviour was hardcoded per verb.** A puzzle ("cut the rope") meant
  overriding `on_use` wholesale, re-implementing the stock cases plus the new
  one. Every custom interaction was a fork, not an addition.
- **The lock/door loop wasn't proven from the consumer side.** `ExtraData` had
  a key named `opens_with`, but unlocking was neither data-driven nor tested.

We researched how classic engines solve this. Inform 7 / TADS unify *every*
interactable into one "thing" and resolve nouns across a scope. Visionaire
(and AGS) go further: **there are exactly two object kinds — scene objects and
inventory objects**; verbs are *data*, interactions are authored per object,
everything has a fallback answer, and the GUI can enumerate interactions. The
rewrite follows the Visionaire path, because it maps most directly onto
requirement #3.

## What changed

### 1. One interactable: an `Object`, with exactly two kinds

Every interactable is an `Object` with an id, names and opaque `extra`.
An object has one of two kinds:

- `Item` — an inventory object: portable, taken into inventory (a key, a sword).
  This is the default kind when world data omits it.
- `Scene` — a scene object: stays in the world, clickable/examinable but never
  portable (furniture, fixtures… and every door).

A **door is just a Scene object carrying optional door data**:

```yaml
- id: 14
  primary_name: oak door
  aliases: [door]
  kind: Scene
  door:
    direction: east
    to: 2
    locked: true
    gated_by: 2
```

So the three `ObjectTarget`s of the interim design (`Item | Exit`) are gone:
`resolve_target(name)` now resolves one noun path against visible room objects
and carried objects, and returns `Found(ObjectId)` / `Ambiguous { ids }` /
`NotFound`. Declaring `door` data forces the kind to `Scene`, keeping the
two-kind invariant at runtime.

### 2. Rooms hold objects; the direction index is derived

`Room` no longer stores an `exits:` map. It holds `objects`, `hidden_objects`
and a *derived* `HashMap<Direction, ObjectId>` built once from the door objects
in both lists — a cache for O(1) movement, not an identity store. "Hidden" is
*list membership*, not a flag on the door: a hidden door is an object sitting in
`hidden_objects` until revealed. The `locked` flag stays on the door data.

The old exit helpers survive as door helpers over the index:
`get_room_id_by_exit_direction` (open → destination), `is_exit_locked` /
`is_exit_hidden`, `exit_directions`, `exit_extra`, `unlock_exit` / `lock_exit`,
`reveal_exit` / `hide_exit`. `reveal_object` and `reveal_exit` are now the same
operation (move an object between `hidden_objects` and `objects`).

### 3. The scene-vs-inventory distinction is stock behaviour

`BasicRules::on_take` is where the kind distinction bites (Visionaire's authored
"is this portable" behaviour):

- `Item` in the room → taken into inventory (`Took`),
- `Scene` (including every door) → refused with `CantTake { object }` — the
  player can't carry it,
- missing/ambiguous → the usual not-found/ambiguous events.

Because *taking* is now default behaviour, the front-end's old `TakeRules`
override is gone; the game only authors a `GameRules::on_look` that reveals the
hidden passage door.

### 4. Interactions are authored, and the fallback is stock

Behaviour is a small composition:

- A **`Rules` trait hook** (`on_use`, `on_take`, ...) is the coarse overridable
  layer.
- In addition, a consumer can provide **`Interaction`s** (verb + item + target
  filter + condition + effect) via `Rules::interactions()`.
- The **default `on_use`** first runs any matching authored interaction, then
  falls back to a stock spine:

  - a resolved target that is a door whose `gated_by` equals the used object →
    `UnlockedExit` (door-ness detected backwards, via `exit_direction_of(target)`),
  - use-on-object → `Used`,
  - resolved-but-meaningless combinations → `CannotUse`,
  - missing/ambiguous parts → `UsedTargetNeeded` / `UsedTargetNotFound` /
    `UsedTargetAmbiguous`.

  So a puzzle authors *one* interaction ("cut the rope with the knife") and the
  engine answers for *everything else* without the author re-implementing
  refusal cases.

The gated door is now **data + a stock rule**: a door object declares
`gated_by: <object id>` (a pure fact), and the default `on_use` turns "use the
gating object on the locked door" into an unlock. The front-end needs no custom
unlock code.

`TargetFilter` names the two kinds: `Any`, `Targeted`, `Scene`, `Door` — the
door filter is how an interaction addresses "any door", with finer selection in
the `condition`.

### 5. The point-and-click hook: `GameEngine::interactions_for`

A GUI asks "what can the player do with this target right now?":

```rust
engine.interactions_for(Some(iron_key_id), Some(oak_door_id))
```

The query runs the *same* `matches()` (verb + item + target filter + condition)
that the dispatcher uses, so it only reports interactions that are currently
valid — no duplicate logic for the menu vs. the execution. Stock `BasicRules`
reports nothing here; the query lists *authored* interactions, which a GUI
combines with raw world state (e.g. `WorldState::exit_gated_by`) to compose its
verb menu.

### 6. Events: object-flavoured and open

- All payloads carry object ids/names (`TookObjectNotFound`, `Dropped`, `Used`,
  `Examined`, ...).
- New: `UnlockedExit { direction }`, `CannotUse { item, target }` (the generic
  refusal), `CantTake { object }` (scene objects are not portable), and
  `Event::Custom { name }` — an opaque, game-authored beat (the engine's
  "plugin channel", from the backlog).

## The pipeline today

```
text input → tokenizer → lexer → Action
  → GameEngine.handle_input → execute_action
      → resolve_target / resolve_player_object / resolve_room_object
      → Rules hook
          → authored Interaction? (conditions re-checked)
          → else stock fallback (kind check, gated unlock, refusals)
      → Vec<Event> + &mut WorldState
  → View.render → text output
```

A point-and-click front-end is just a View plus the `interactions_for` query:
it synthesizes an `Action` from clicks, sends it through the exact same
`execute_action` path, and renders `Event`s.

## Code map

| File | Role |
| --- | --- |
| `crates/core/src/world/object.rs` | `ObjectId`, `ObjectKind`, `ObjectResolution` (plain ids), `Object` (kind + door state), `ObjectInfo` (with `DoorInfo`) |
| `crates/core/src/world/room.rs` | `Room`: object/hidden-object lists, derived `Direction → ObjectId` index, door helpers (lock/hide/reveal/gate/extra) |
| `crates/core/src/world/mod.rs` | `WorldState`: unified `resolve_target`, scope helpers, `object_kind`/`object_is_door`/`exit_direction_of`, transfers, `from_data` |
| `crates/core/src/interaction.rs` | `Verb`, `ActionContext` (option ids), `TargetFilter` (Any/Targeted/Scene/Door), `Interaction` |
| `crates/core/src/rules.rs` | `BasicRules`, `Rules` (defaults): `on_take` kind check, stock `on_use` with interaction dispatch + gated unlock |
| `crates/core/src/engine.rs` | Action → hook dispatch, `interactions_for` query |
| `crates/core/src/event.rs` | `Event` enum (object payloads, `UnlockedExit`, `CannotUse`, `CantTake`, `Custom`) |
| `crates/core/src/data.rs` | `WorldData`/`ObjectData`/`RoomData`, `ObjectKind`, `DoorData` (`direction`, `to`, `locked`, `gated_by`) |
| `src/main.rs`, `src/view.rs` | `GameRules` (only the reveal-on-look beat), `TextView` |

## Data mapping

- `items:` → `objects:`; every object may declare `kind: Item | Scene`
  (default `Item`).
- `visible_items:` → `visible_objects:`; `hidden_items:` → `hidden_objects:`.
- Per-room `exits:` maps are gone — a door is an object with a `door:` block:

```yaml
- id: 14
  primary_name: oak door
  aliases: [door]
  kind: Scene
  door:
    direction: east
    to: 2
    locked: true
    gated_by: 2
  extra:
    material: oak
```

`WorldData::from_yaml` / `WorldData::load` signatures are unchanged; only the
file format moved.

## One important design note

Authored interactions and defaults must live on the **same concrete `Rules`
type**. `BasicRules::on_use` consults `interactions()` so a consumer type that
provides interactions gets the stock fallback *plus* its additions for free —
but only when that type is what the engine dispatches on. A wrapper that
delegates `on_use` to a plain `BasicRules` silently loses the wrapper's
interactions (the default runs against `BasicRules`). Provide `interactions()`
on your own `Rules` impl instead of nesting.

## Follow-ups

- Both original backlog items are effectively closed: `Event::Custom` is the
  "opaque game event passthrough", and the gated door is proven end-to-end from
  the consumer side (the stock `on_use` unlock, exercised by the game).
- Natural next steps: a GUI front-end (View + `interactions_for` menus), and
  dialogue/"use on hidden door to reveal it" scenarios encoded as interactions.