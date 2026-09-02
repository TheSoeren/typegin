//! Tests for the input pipeline, exercised through the public `parse_input`.
//!
//! The internal `tokenize` / `lex` functions and the `input` module are not
//! part of the public API (only `parse_input` is re-exported), so these cases
//! cover the same behaviour through the public entry point.
//!
//! Run with: cd crates/core && cargo test --test input

use core::{Action, Direction, parse_input};

fn parses(input: &str) -> Action {
    parse_input(input)
}

mod parse {
    use super::*;

    #[test]
    fn look() {
        assert_eq!(Action::Look, parses("look"));
    }

    #[test]
    fn look_shortcut() {
        assert_eq!(Action::Look, parses("l"));
    }

    #[test]
    fn go_directions() {
        assert_eq!(Action::Go(Direction::North), parses("go north"));
        assert_eq!(Action::Go(Direction::East), parses("go east"));
        assert_eq!(Action::Go(Direction::South), parses("go south"));
        assert_eq!(Action::Go(Direction::West), parses("go west"));
    }

    #[test]
    fn direction_shortcuts() {
        assert_eq!(Action::Go(Direction::North), parses("n"));
        assert_eq!(Action::Go(Direction::East), parses("e"));
        assert_eq!(Action::Go(Direction::South), parses("s"));
        assert_eq!(Action::Go(Direction::West), parses("w"));
    }

    #[test]
    fn bare_direction_words() {
        assert_eq!(Action::Go(Direction::North), parses("north"));
        assert_eq!(Action::Go(Direction::East), parses("east"));
    }

    #[test]
    fn examine() {
        assert_eq!(
            Action::Examine("glowing sword".to_string()),
            parses("examine the glowing sword")
        );
    }

    #[test]
    fn examine_shortcut() {
        assert_eq!(Action::Examine("chest".to_string()), parses("x chest"));
    }

    #[test]
    fn take() {
        assert_eq!(
            Action::Take("heavy iron key".to_string()),
            parses("take the heavy iron key")
        );
    }

    #[test]
    fn take_shortcut() {
        assert_eq!(Action::Take("lantern".to_string()), parses("get lantern"));
    }

    #[test]
    fn drop() {
        assert_eq!(
            Action::Drop("iron key".to_string()),
            parses("drop the iron key")
        );
    }

    #[test]
    fn drop_shortcut() {
        assert_eq!(Action::Drop("sword".to_string()), parses("d sword"));
    }

    #[test]
    fn drop_empty_is_unknown() {
        assert_eq!(Action::Unknown("drop".to_string()), parses("drop"));
    }

    #[test]
    fn use_on_target() {
        assert_eq!(
            Action::Use {
                item: "brass key".to_string(),
                target: Some("wooden door".to_string()),
            },
            parses("use brass key on wooden door")
        );
    }

    #[test]
    fn use_with_alias() {
        assert_eq!(
            Action::Use {
                item: "wrench".to_string(),
                target: Some("bolt".to_string()),
            },
            parses("use wrench with bolt")
        );
    }

    #[test]
    fn use_single_item() {
        assert_eq!(
            Action::Use {
                item: "potion".to_string(),
                target: None,
            },
            parses("use potion")
        );
    }

    #[test]
    fn use_empty_is_unknown() {
        assert_eq!(Action::Unknown("use".to_string()), parses("use"));
    }

    #[test]
    fn unknown_verb() {
        assert_eq!(
            Action::Unknown("dance wildly".to_string()),
            parses("dance wildly")
        );
    }

    #[test]
    fn unknown_direction() {
        assert_eq!(
            Action::Unknown("go sideways".to_string()),
            parses("go sideways")
        );
    }

    #[test]
    fn empty_input() {
        assert_eq!(Action::Unknown("".to_string()), parses(""));
    }

    #[test]
    fn mixed_case_and_punctuation() {
        assert_eq!(
            Action::Examine("glowing mysterious sword".to_string()),
            parses("  EXAMINE the glowing, mysterious sword! ")
        );
    }

    #[test]
    fn hyphenated_terms_are_split() {
        assert_eq!(
            Action::Unknown("open brass key door".to_string()),
            parses("open the brass-key door")
        );
    }

    #[test]
    fn whitespace_is_collapsed() {
        assert_eq!(
            Action::Examine("chest".to_string()),
            parses("examine   the   chest")
        );
    }
}
