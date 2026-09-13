# Examples

A workspace-level directory holding the two front-ends and the one game
world they both play, grouped here rather than nested under `crates/`
because `crates/core` and `crates/render` are meant to stay reusable engine
libraries with no game content or front-end of their own — see `AGENTS.md`.

## `data/`

The one shared world both front-ends read: a small homage to the opening
cell of _Edna & Harvey: The Breakout_. One copy, split across
`globals.yaml` / `items.yaml` / `rooms.yaml` / `interactions.yaml` /
`npcs.yaml` / `triggers.yaml` for authoring convenience — every consumer
concatenates them into one YAML document before handing it to
`core::WorldData::from_yaml`.

Some objects in `items.yaml` also carry `extra.gui.hotspot` data (an
`{x, y, w, h}` rectangle) — that's `pnc`'s own convention for where an
object sits on screen, described in [`crates/render`](../crates/render).
`core` never interprets it, and `text` never reads it; it's purely
additive.

## `text/` — `cargo run -p text-sample`

The maintained terminal front-end (binary name `text-proto`): reads the
shared data, plays it with hand-written prose (`src/view.rs`), and supports
`save`/`load`.

## `pnc/` — `cargo run -p pnc-sample`

A real (if minimal) macroquad window exercising `crates/render` against the
same shared data. Proves the render engine's actual wiring.

- An object currently in the room but missing authored `gui.hotspot` data
  still shows up, listed as plain text at the bottom of the window, instead
  of silently vanishing.

Known rough edges, not bugs so much as "the next thing to build":

- The object list it hit-tests against is hardcoded in `main.rs`
  (`candidate_object_ids`) — `core` doesn't yet expose a "list this room's
  objects" query, only lookups by an id you already have.
- Clicking always fires the coin's _first_ (priority-ordered) verb, not
  whichever one is actually under the cursor — there's no per-entry angle
  picking yet.
- No asset-provider extension point yet, so there's no way for a real game
  to plug in its own art; see `crates/render`'s design notes in `AGENTS.md`.
