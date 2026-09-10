use crate::event::Event;
use crate::input::direction::Direction;
use crate::world::WorldState;

mod render_command;

pub use render_command::RenderCommand;

/// Turns gameplay events into screen output.
///
/// This is the *outbound* half of the engine: it observes [`Event`]s and
/// [`WorldState`] read-only, after [`Rules`](crate::rules::Rules) has already
/// decided what happened. Implement it once per output style (terminal, GUI,
/// log, translations, ...) and swap it on a UI; [`RenderCommand`] lets text
/// and GUI front-ends share one pipeline, though a GUI may also read
/// [`Event`]s and [`WorldState`] directly and draw itself.
///
/// Override one `render_*` hook per event you want to phrase; the default
/// [`View::render`] dispatches each event to its hook with the payload
/// already destructured, and falls back to [`View::render_generic`] for a
/// not-yet-dispatched event, so a new engine [`Event`] never breaks an
/// existing view. Hooks take `&mut self` so a stateful view can pace output
/// or accumulate a transcript.
pub trait View {
    /// Render a batch of events into screen commands, in order.
    ///
    /// The default matches each event against its typed `render_*` hook.
    /// Override only if you need to combine events (e.g. collapse
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
                Event::Granted { object, .. } => self.render_granted(object),
                Event::Dropped { object, .. } => self.render_dropped(object),
                Event::DroppedObjectNotFound { object } => {
                    self.render_dropped_object_not_found(object)
                }
                Event::DroppedObjectAmbiguous { object, .. } => {
                    self.render_dropped_object_ambiguous(object)
                }
                Event::Discarded { object, .. } => self.render_discarded(object),
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
                Event::Examined {
                    target_name: object,
                    ..
                } => self.render_examined(object),
                Event::ExaminedTargetNotFound { target: object } => {
                    self.render_examined_object_not_found(object)
                }
                Event::ExaminedTargetAmbiguous { target: object, .. } => {
                    self.render_examined_object_ambiguous(object)
                }
                Event::UnknownEvent { name } => self.render_unknown_event(name),
                Event::FlagSet { flag } => self.render_flag_set(flag),
                Event::FlagCleared { flag } => self.render_flag_cleared(flag),
                Event::Talked {
                    npc, text, choices, ..
                } => self.render_talked(npc, text, choices),
                Event::DialogueEnded { npc, .. } => self.render_dialogue_ended(npc),
                Event::TalkNpcNotFound { npc } => self.render_talk_npc_not_found(npc),
                Event::DialogueInvalidChoice { npc, choice } => {
                    self.render_dialogue_invalid_choice(npc, choice)
                }
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

    /// A locked exit was unlocked.
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

    /// An object was granted into inventory, regardless of where (if
    /// anywhere) it was placed in the world.
    fn render_granted(&mut self, _object: &str) -> Vec<RenderCommand> {
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

    /// An object was removed from inventory without being placed anywhere.
    fn render_discarded(&mut self, _object: &str) -> Vec<RenderCommand> {
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

    /// A global flag was set.
    fn render_flag_set(&mut self, _flag: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// A global flag was cleared.
    fn render_flag_cleared(&mut self, _flag: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// An NPC spoke; `choices` are the player's available responses.
    fn render_talked(
        &mut self,
        _npc: &str,
        _text: &str,
        _choices: &[crate::event::DialogueChoice],
    ) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// A dialogue conversation ended.
    fn render_dialogue_ended(&mut self, _npc: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player tried to talk to an NPC that is not in the current room.
    fn render_talk_npc_not_found(&mut self, _npc: &str) -> Vec<RenderCommand> {
        Vec::new()
    }

    /// The player picked a dialogue option that does not exist.
    fn render_dialogue_invalid_choice(&mut self, _npc: &str, _choice: &str) -> Vec<RenderCommand> {
        Vec::new()
    }
}
