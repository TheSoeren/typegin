//! `render`: a prototype point-and-click front-end for `typegin_core`,
//! built on `macroquad`.
//!
//! This crate does not need to serve the text or text+GUI modalities it exists to build whatever a
//! `PnC` UI specifically needs: hotspots, a radial verb coin, drag-style combine.
pub mod asset;
pub mod coin;
mod extra;
pub mod hotspot;
pub mod pathfinding;
pub mod tween;
