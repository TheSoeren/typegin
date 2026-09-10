# TODO

## Engine gaps vs. a 1:1 replica of Edna & Harvey: The Breakout — first room

The first-room (padded cell) puzzle is now replicated in `data/` as faithfully
as the current engine allows. Everything below could NOT be replicated and is
a candidate core feature (each is general to adventure games, not E&H-specific).

### Replicated (works today, in `data/{items,rooms,interactions,npcs}.yaml`)

1. Examine the chair → loose chair leg is found.
2. Break the chair leg over the table → broken chair leg.
3. Rip the padding with the broken chair leg → the distinctive torn pad.
4. Charm the guard through the door grate → **air conditioning ON** → the torn
   pad falls away, exposing the **airy gap**.
5. Open the airy gap with the broken chair leg → **vent grille + 4 screws**.
6. Get a **toenail** → unscrew the grille → **fan** exposed.
7. Provoke the guard → **air conditioning OFF** → remove the now-still fan →
   crawl into the ventilation shaft.

### Engine gaps (cannot be replicated — future core features)

1. ~~**Inventory-companion NPC**~~ — **Done.** `examine`/`use`/`talk` are now
   target-oriented (`WorldState::resolve_target` resolves a name to either
   `Target::Npc` or `Target::Object`), so `use <item> on <npc>` works the same
   way as `use <item> on <object>`. Harvey-as-companion is still not
   representable (NPCs remain room-bound, not carryable), but the verb gap
   that blocked it is closed.
2. **Verbless scene manipulation (push/operate)** — removing the fan is
   "Use the fan; Edna asks Harvey to help push it" with no inventory item.
   The engine's verbs are take/drop/examine/use/go/talk/choose; every `use`
   requires a carried item and an optional scene target. Standing proxy: use
   the broken chair leg on the fan.
3. **Flag-gated dialogue choices** — the mean ("whack it across your skull")
   vs. nice guard options should only appear in the right conversation state;
   dialogue choices cannot be gated on flags/conditions, so both are always
   offered. (Flag-gated _room interactions_ work; only _choices_ lack
   conditions.)
4. **Environmental / ambient state** — heat, AC on/off is flavour-only (flags
   - prose). Survival-relevant state is not a first-class concept, though
     flag-gated descriptions cover the visible consequence here.
5. **Cosmetic multiplicity** — the 19 pads collapse to one _Padded wall_
   object (the meaningful third-from-top pad). The engine has one object per
   noun; dozens of identical-looking pads are not modelled.
