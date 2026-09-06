use crate::event::Event;
use crate::input::direction::Direction;
use crate::world::WorldState;

/// A screen-level instruction produced by a [`View`].
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

/// Turns gameplay events into screen output.
///
/// The engine only ever produces typed [`Event`]s and mutates
/// [`WorldState`]; turning those into visible output is entirely this
/// trait's job. Implement it once per output style (terminal, web, log,
/// translations, ...) and swap it on a UI. A GUI that reads the [`Event`]s
/// and [`WorldState`] directly and draws itself also gets the same shared,
/// read-only access — [`RenderCommand`] merely lets text and GUI front-ends
/// share one pipeline.
///
/// This is the *outbound* hook: it observes events read-only, after the
/// game logic has already run. Inbound decision-making lives in [`Rules`].
///
/// Idiomatic use: override one [`View::render_*`] hook per event you want to
/// phrase. The default [`View::render`] dispatches every event to its hook
/// with the payload already destructured (names, directions, ...), so a view
/// only implements what it phrases. A not-yet-dispatched event falls back to
/// [`View::render_generic`], so a new engine [`Event`] never breaks an
/// existing view.
///
/// Hooks take `&mut self` so stateful views can pace output, accumulate a
/// transcript, or animate; a pure view simply ignores the mutation.
///
/// [`Rules`]: crate::engine::Rules
pub trait View {
    /// Render a batch of events into screen commands, in order.
    ///
    /// The default matches each event against its typed [`View::render_*`]
    /// hook. Override only if you need to combine events (e.g. collapse
    /// consecutive notifications).
    fn render(&mut self, events: &[Event], world: &WorldState) -> Vec<RenderCommand> {
        #[allow(unreachable_patterns)]
        // fallback: a not-yet-mapped event still reaches the generic hook
        events
            .iter()
            .flat_map(|event| match event {
                Event::Looked => self.render_looked(world),
                Event::Went(direction) => self.render_went(direction),
                Event::WentExitHidden(direction) => self.render_went_exit_hidden(direction),
                Event::WentExitLocked(direction) => self.render_went_exit_locked(direction),
                Event::WentInvalidDirection(direction) => {
                    self.render_went_invalid_direction(direction)
                }
                Event::UnlockedExit { direction } => self.render_unlocked_exit(direction),
                Event::CannotUse { item, target } => self.render_cannot_use(item, target),
                Event::Custom { name } => self.render_custom(name),
                Event::Took { object, .. } => self.render_took(object),
                Event::TookObjectNotFound { object } => self.render_took_object_not_found(object),
                Event::TookObjectAmbiguous { object, .. } => {
                    self.render_took_object_ambiguous(object)
                }
                Event::CantTake { object } => self.render_cant_take(object),
                Event::Dropped { object, .. } => self.render_dropped(object),
                Event::DroppedObjectNotFound { object } => {
                    self.render_dropped_object_not_found(object)
                }
                Event::DroppedObjectAmbiguous { object, .. } => {
                    self.render_dropped_object_ambiguous(object)
                }
                Event::Used { object, target, .. } => self.render_used(object, target.as_deref()),
                Event::UsedObjectNotFound { object } => self.render_used_object_not_found(object),
                Event::UsedObjectAmbiguous { object, .. } => {
                    self.render_used_object_ambiguous(object)
                }
                Event::UsedTargetNeeded { object, .. } => self.render_used_target_needed(object),
                Event::UsedTargetNotFound { object, target, .. } => {
                    self.render_used_target_not_found(object, target)
                }
                Event::UsedTargetAmbiguous { object, .. } => {
                    self.render_used_target_ambiguous(object)
                }
                Event::Examined { object, .. } => self.render_examined(object),
                Event::ExaminedObjectNotFound { object } => {
                    self.render_examined_object_not_found(object)
                }
                Event::ExaminedObjectAmbiguous { object, .. } => {
                    self.render_examined_object_ambiguous(object)
                }
                Event::UnknownEvent { name } => self.render_unknown_event(name),
                other => self.render_generic(other),
            })
            .collect()
    }

    /// Fallback for an [`Event`] the dispatcher does not map yet.
    ///
    /// Defaults to silence, so new engine events never break existing views;
    /// override to e.g. echo the event for debugging.
    fn render_generic(&mut self, _event: &Event) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player looked around; `world` reflects the current room.
    fn render_looked(&mut self, _world: &WorldState) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player moved through a passable exit.
    fn render_went(&mut self, _direction: &Direction) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to go through a hidden exit.
    fn render_went_exit_hidden(&mut self, _direction: &Direction) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to go through a locked exit.
    fn render_went_exit_locked(&mut self, _direction: &Direction) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to go through an exit that does not exist.
    fn render_went_invalid_direction(&mut self, _direction: &Direction) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// A locked exit was unlocked by using its `gated_by` object on it.
    fn render_unlocked_exit(&mut self, _direction: &Direction) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// An interaction resolved but makes no sense ("use sword on the open door").
    fn render_cannot_use(&mut self, _item: &str, _target: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// A game-authored beat emitted via a [`Event::Custom`] interaction effect.
    fn render_custom(&mut self, _name: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player took an item into inventory.
    fn render_took(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to take an item they cannot see.
    fn render_took_object_not_found(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player referenced an object that matches several visible objects.
    fn render_took_object_ambiguous(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to take a scene object, which is not portable.
    fn render_cant_take(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player dropped an item from inventory.
    fn render_dropped(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to drop an item they are not carrying.
    fn render_dropped_object_not_found(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player referenced an item matching several carried items.
    fn render_dropped_object_ambiguous(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player used one object, optionally on a target.
    fn render_used(&mut self, _object: &str, _target: Option<&str>) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to use an item they do not have.
    fn render_used_object_not_found(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player referenced an item matching several carried items to use.
    fn render_used_object_ambiguous(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player used an item but gave no target for a target-requiring verb.
    fn render_used_target_needed(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player used an item on a target that does not exist.
    fn render_used_target_not_found(&mut self, _object: &str, _target: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player's target matched several visible objects.
    fn render_used_target_ambiguous(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player examined an object.
    fn render_examined(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to examine an object that does not exist.
    fn render_examined_object_not_found(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player referenced an object matching several to examine.
    fn render_examined_object_ambiguous(&mut self, _object: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player typed an unknown command.
    fn render_unknown_event(&mut self, _name: &str) -> Vec<RenderCommand> {
        Vec::new()
    }
}
