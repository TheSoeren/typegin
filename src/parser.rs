use crate::action::Action;
use crate::direction::Direction;

const USE_WORDS: &[&str] = &["on", "with"];

pub fn lex(tokens: &Vec<&str>) -> Action {
    match tokens.as_slice() {
        ["look" | "l"] => Action::Look,
        ["examine" | "x", rest @ ..] => Action::Examine(rest.join(" ")),
        ["take" | "get", rest @ ..] => Action::Take(rest.join(" ")),
        ["use", rest @ ..] => match get_use(rest) {
            Some(action) => action,
            None => Action::Unknown("use".to_string()),
        },
        ["go", direction] => match Direction::parse(direction) {
            Some(d) => Action::Go(d),
            None => Action::Unknown(direction.to_string()),
        },
        [direction] => match Direction::parse(direction) {
            Some(d) => Action::Go(d),
            None => Action::Unknown(direction.to_string()),
        },
        _ => Action::Unknown("".to_string()),
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
