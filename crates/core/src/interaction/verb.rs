use serde::Deserialize;

use crate::input::action::Action;

/// The game's action vocabulary. An author writes interactions *for a verb*,
/// and a point-and-click front-end can enumerate the verbs an object accepts
/// instead of guessing from prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verb {
    Look,
    Go,
    Examine,
    Take,
    Drop,
    Use,
    Talk,
}

impl Verb {
    /// The verb a parsed [`Action`] maps to, when it maps to one at all.
    ///
    /// `Unknown` actions have no verb; `Use` with or without a target is the
    /// same verb (the target lives in the context, not the verb).
    #[must_use]
    pub fn from_action(action: &Action) -> Option<Verb> {
        match action {
            Action::Look => Some(Verb::Look),
            Action::Go(_) => Some(Verb::Go),
            Action::Examine(_) => Some(Verb::Examine),
            Action::Take(_) => Some(Verb::Take),
            Action::Drop(_) => Some(Verb::Drop),
            Action::Use { .. } => Some(Verb::Use),
            Action::Talk(_) => Some(Verb::Talk),
            Action::Choose(_) | Action::Unknown(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::direction::Direction;

    #[test]
    fn maps_each_actionable_variant_to_its_verb() {
        assert_eq!(Verb::from_action(&Action::Look), Some(Verb::Look));
        assert_eq!(
            Verb::from_action(&Action::Go(Direction::North)),
            Some(Verb::Go)
        );
        assert_eq!(
            Verb::from_action(&Action::Examine("sword".to_string())),
            Some(Verb::Examine)
        );
        assert_eq!(
            Verb::from_action(&Action::Take("sword".to_string())),
            Some(Verb::Take)
        );
        assert_eq!(
            Verb::from_action(&Action::Drop("sword".to_string())),
            Some(Verb::Drop)
        );
        assert_eq!(
            Verb::from_action(&Action::Talk("guard".to_string())),
            Some(Verb::Talk)
        );
    }

    #[test]
    fn use_maps_to_the_same_verb_with_or_without_a_target() {
        assert_eq!(
            Verb::from_action(&Action::Use {
                item: "key".to_string(),
                target: None,
            }),
            Some(Verb::Use)
        );
        assert_eq!(
            Verb::from_action(&Action::Use {
                item: "key".to_string(),
                target: Some("door".to_string()),
            }),
            Some(Verb::Use)
        );
    }

    #[test]
    fn choose_and_unknown_have_no_verb() {
        assert_eq!(Verb::from_action(&Action::Choose("1".to_string())), None);
        assert_eq!(
            Verb::from_action(&Action::Unknown("dance".to_string())),
            None
        );
    }
}
