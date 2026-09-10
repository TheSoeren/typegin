/// A screen-level instruction produced by a [`View`](crate::view::View).
///
/// A text front-end turns these into lines on the terminal; a GUI front-end
/// interprets them in whatever widget tree it owns. Because the enum is
/// `#[non_exhaustive]` the engine can add commands later without breaking
/// existing consumers — interpreters must carry a fallback arm.
#[non_exhaustive]
pub enum RenderCommand {
    /// One line of prose for a terminal or transcript.
    Line(String),
    /// Wipe the current view (e.g. before showing new [`RenderCommand::Line`]s).
    ClearScreen,
}
