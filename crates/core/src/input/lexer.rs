use super::action::Action;
use super::direction::Direction;

const USE_WORDS: &[&str] = &["on", "with"];

pub fn lex(tokens: &[&str]) -> Action {
    match tokens {
        ["look" | "l"] => Action::Look,
        ["examine" | "x", rest @ ..] => match !rest.is_empty() {
            true => Action::Examine(rest.join(" ")),
            false => Action::Unknown(tokens.join(" ")),
        },
        ["take" | "get", rest @ ..] => match !rest.is_empty() {
            true => Action::Take(rest.join(" ")),
            false => Action::Unknown(tokens.join(" ")),
        },
        ["drop" | "d", rest @ ..] => match !rest.is_empty() {
            true => Action::Drop(rest.join(" ")),
            false => Action::Unknown(tokens.join(" ")),
        },
        ["use", rest @ ..] => match get_use(rest) {
            Some(action) => action,
            None => Action::Unknown("use".to_string()),
        },
        ["go", direction] => match Direction::parse(direction) {
            Some(d) => Action::Go(d),
            None => Action::Unknown(tokens.join(" ")),
        },
        [direction] => match Direction::parse(direction) {
            Some(d) => Action::Go(d),
            None => Action::Unknown(tokens.join(" ")),
        },
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
