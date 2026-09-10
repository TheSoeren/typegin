use super::action::Action;
use super::direction::Direction;

const USE_WORDS: &[&str] = &["on", "with"];

/// Turn already-tokenized words into an [`Action`]; unrecognised input becomes
/// [`Action::Unknown`].
#[must_use]
pub fn lex(tokens: &[&str]) -> Action {
    // The `go <direction>` and bare `<direction>` arms share a body but differ
    // in arity (two tokens vs one), so they cannot be merged into one pattern.
    #[expect(clippy::match_same_arms)]
    match tokens {
        ["look" | "l"] => Action::Look,
        ["examine" | "x", rest @ ..] => {
            if rest.is_empty() {
                Action::Unknown(tokens.join(" "))
            } else {
                Action::Examine(rest.join(" "))
            }
        }
        ["take" | "get", rest @ ..] => {
            if rest.is_empty() {
                Action::Unknown(tokens.join(" "))
            } else {
                Action::Take(rest.join(" "))
            }
        }
        ["drop" | "d", rest @ ..] => {
            if rest.is_empty() {
                Action::Unknown(tokens.join(" "))
            } else {
                Action::Drop(rest.join(" "))
            }
        }
        ["use", rest @ ..] => match get_use(rest) {
            Some(action) => action,
            None => Action::Unknown("use".to_string()),
        },
        ["go", direction] => direction_to_action(direction, tokens),
        [direction] => direction_to_action(direction, tokens),
        ["talk", rest @ ..] => {
            if rest.is_empty() {
                Action::Unknown(tokens.join(" "))
            } else {
                Action::Talk(rest.join(" "))
            }
        }
        ["choose", rest @ ..] => {
            if rest.is_empty() {
                Action::Unknown(tokens.join(" "))
            } else {
                Action::Choose(rest.join(" "))
            }
        }
        _ => Action::Unknown(tokens.join(" ")),
    }
}

fn split_use_target<'a>(list: &'a [&'a str]) -> Option<(&'a [&'a str], &'a [&'a str])> {
    if let Some(index) = list.iter().position(|item| USE_WORDS.contains(item)) {
        let before = &list[..index];
        let after = &list[index + 1..];
        Some((before, after))
    } else {
        None
    }
}

fn get_use(rest: &[&str]) -> Option<Action> {
    if rest.is_empty() {
        return None;
    }

    match split_use_target(rest) {
        Some((item, target)) => Some(Action::Use {
            item: item.join(" "),
            target: Some(target.join(" ")),
        }),
        None => Some(Action::Use {
            item: rest.join(" "),
            target: None,
        }),
    }
}

fn direction_to_action(direction: &str, tokens: &[&str]) -> Action {
    match Direction::parse(direction) {
        Some(d) => Action::Go(d),
        None => Action::Unknown(tokens.join(" ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_and_shortcut() {
        assert_eq!(lex(&["look"]), Action::Look);
        assert_eq!(lex(&["l"]), Action::Look);
    }

    #[test]
    fn examine_joins_remaining_tokens() {
        assert_eq!(
            lex(&["examine", "glowing", "sword"]),
            Action::Examine("glowing sword".to_string())
        );
        assert_eq!(lex(&["x", "chest"]), Action::Examine("chest".to_string()));
    }

    #[test]
    fn examine_with_no_object_is_unknown() {
        assert_eq!(lex(&["examine"]), Action::Unknown("examine".to_string()));
        assert_eq!(lex(&["x"]), Action::Unknown("x".to_string()));
    }

    #[test]
    fn take_and_get_alias() {
        assert_eq!(lex(&["take", "key"]), Action::Take("key".to_string()));
        assert_eq!(
            lex(&["get", "lantern"]),
            Action::Take("lantern".to_string())
        );
    }

    #[test]
    fn take_with_no_object_is_unknown() {
        assert_eq!(lex(&["take"]), Action::Unknown("take".to_string()));
    }

    #[test]
    fn drop_and_shortcut() {
        assert_eq!(lex(&["drop", "sword"]), Action::Drop("sword".to_string()));
        assert_eq!(lex(&["d", "sword"]), Action::Drop("sword".to_string()));
    }

    #[test]
    fn drop_with_no_object_is_unknown() {
        assert_eq!(lex(&["drop"]), Action::Unknown("drop".to_string()));
    }

    #[test]
    fn use_with_on_target() {
        assert_eq!(
            lex(&["use", "brass", "key", "on", "wooden", "door"]),
            Action::Use {
                item: "brass key".to_string(),
                target: Some("wooden door".to_string()),
            }
        );
    }

    #[test]
    fn use_with_with_alias() {
        assert_eq!(
            lex(&["use", "wrench", "with", "bolt"]),
            Action::Use {
                item: "wrench".to_string(),
                target: Some("bolt".to_string()),
            }
        );
    }

    #[test]
    fn use_first_matched_keyword_wins_when_both_present() {
        // "on" appears before "with"; the first match splits item/target.
        assert_eq!(
            lex(&["use", "key", "on", "chest", "with", "lock"]),
            Action::Use {
                item: "key".to_string(),
                target: Some("chest with lock".to_string()),
            }
        );
    }

    #[test]
    fn use_single_item_no_target() {
        assert_eq!(
            lex(&["use", "potion"]),
            Action::Use {
                item: "potion".to_string(),
                target: None,
            }
        );
    }

    #[test]
    fn use_with_no_item_is_unknown() {
        assert_eq!(lex(&["use"]), Action::Unknown("use".to_string()));
    }

    #[test]
    fn go_direction_full_word_and_abbreviation() {
        assert_eq!(lex(&["go", "north"]), Action::Go(Direction::North));
        assert_eq!(lex(&["go", "n"]), Action::Go(Direction::North));
    }

    #[test]
    fn go_unknown_direction_is_unknown() {
        assert_eq!(
            lex(&["go", "sideways"]),
            Action::Unknown("go sideways".to_string())
        );
    }

    #[test]
    fn bare_direction_word() {
        assert_eq!(lex(&["north"]), Action::Go(Direction::North));
        assert_eq!(lex(&["e"]), Action::Go(Direction::East));
    }

    #[test]
    fn talk_joins_remaining_tokens() {
        assert_eq!(
            lex(&["talk", "old", "man"]),
            Action::Talk("old man".to_string())
        );
    }

    #[test]
    fn talk_with_no_target_is_unknown() {
        assert_eq!(lex(&["talk"]), Action::Unknown("talk".to_string()));
    }

    #[test]
    fn choose_joins_remaining_tokens() {
        assert_eq!(lex(&["choose", "1"]), Action::Choose("1".to_string()));
        assert_eq!(
            lex(&["choose", "ask", "exit"]),
            Action::Choose("ask exit".to_string())
        );
    }

    #[test]
    fn choose_with_no_payload_is_unknown() {
        assert_eq!(lex(&["choose"]), Action::Unknown("choose".to_string()));
    }

    #[test]
    fn unrecognised_verb_is_unknown() {
        assert_eq!(
            lex(&["dance", "wildly"]),
            Action::Unknown("dance wildly".to_string())
        );
    }

    #[test]
    fn empty_token_list_is_unknown() {
        assert_eq!(lex(&[]), Action::Unknown(String::new()));
    }

    #[test]
    fn split_use_target_finds_first_keyword() {
        assert_eq!(
            split_use_target(&["key", "on", "door"]),
            Some((["key"].as_slice(), ["door"].as_slice()))
        );
    }

    #[test]
    fn split_use_target_returns_none_without_a_keyword() {
        assert_eq!(split_use_target(&["potion"]), None);
    }

    #[test]
    fn get_use_returns_none_for_empty_rest() {
        assert_eq!(get_use(&[]), None);
    }
}
