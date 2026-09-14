//! A visual room/item editor for content authored against `typegin_core`
//! and drawn via `render` — lets `examples/data`-style YAML (hotspots,
//! walkable areas, obstacle holes) be edited by dragging/resizing shapes
//! instead of hand-editing coordinates. Edits go through a real YAML
//! parser (see [`world_yaml`]'s module doc for what that does and doesn't
//! preserve), not ad hoc text scanning.
//!
//! This crate stays generic the same way `core`/`render` do: it edits
//! whatever YAML path/content it's pointed at, no specific game's file
//! layout baked in. `examples/data` is simply its first real target, not
//! an assumption load-bearing in the crate itself.
pub mod world_yaml;
