use super::action::Action;
use super::direction::Direction;

const USE_WORDS: &[&str] = &["on", "with"];

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
