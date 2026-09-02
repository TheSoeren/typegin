use crate::event::Event;
use crate::world::WorldState;

/// Turns gameplay events into display text.
///
/// The engine only ever produces typed `Event`s and mutates `WorldState`;
/// rendering prose to the user is entirely this trait's job. Implement it
/// once per output style (terminal, web, log, translations, ...) and swap
/// it on a UI. A strictly GUI UI doesn't need this at all — it reads the
/// `Event`s and `WorldState` directly.
///
/// This is the *outbound* hook: it observes events read-only, after the game
/// logic has already run. Inbound decision-making lives in [`Rules`], not
/// here — a `View` gets a shared reference only and can never mutate the
/// world.
///
/// [`Rules`]: crate::engine::Rules
pub trait View {
    fn render(&self, events: &[Event], world: &WorldState) -> Vec<String>;
}
