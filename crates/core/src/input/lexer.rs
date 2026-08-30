use super::action::Action;
use super::direction::Direction;

const USE_WORDS: &[&str] = &["on", "with"];

pub fn lex(tokens: &[&str]) -> Action {
    match tokens {
        ["look" | "l"] => Action::Look,
        ["examine" | "x", rest @ ..] => Action::Examine(rest.join(" ")),
        ["take" | "get", rest @ ..] => Action::Take(rest.join(" ")),
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::input::{Action, direction::Direction};

    #[rstest]
    // Single-word verb shortcuts
    #[case::look_shortcut(vec!["look"], Action::Look)]
    #[case::l_shortcut(vec!["l"], Action::Look)]
    // Directional movement & directional shortcuts
    #[case::go_north(vec!["go", "north"], Action::Go(Direction::North))]
    #[case::go_east(vec!["go", "east"], Action::Go(Direction::East))]
    #[case::go_south(vec!["go", "south"], Action::Go(Direction::South))]
    #[case::go_west(vec!["go", "west"], Action::Go(Direction::West))]
    #[case::north_shortcut(vec!["n"], Action::Go(Direction::North))]
    #[case::east_shortcut(vec!["e"], Action::Go(Direction::East))]
    #[case::south_shortcut(vec!["s"], Action::Go(Direction::South))]
    #[case::west_shortcut(vec!["w"], Action::Go(Direction::West))]
    #[case::north_direction(vec!["north"], Action::Go(Direction::North))]
    #[case::east_direction(vec!["east"], Action::Go(Direction::East))]
    #[case::south_direction(vec!["south"], Action::Go(Direction::South))]
    #[case::west_direction(vec!["west"], Action::Go(Direction::West))]
    // Verb + Noun targets (re-joining descriptors)
    #[case::examine_glowing_sword(
        vec!["examine", "glowing", "sword"],
        Action::Examine("glowing sword".to_string())
    )]
    #[case::examine_shortcut(vec!["x", "chest"], Action::Examine("chest".to_string()))]
    #[case::take_heavy_iron_key(
        vec!["take", "heavy", "iron", "key"],
        Action::Take("heavy iron key".to_string())
    )]
    #[case::take_shortcut(vec!["get", "lantern"], Action::Take("lantern".to_string()))]
    // Two-argument interaction ("use <item> on <target>")
    #[case::use_on_target(
        vec!["use", "brass", "key", "on", "wooden", "door"],
        Action::Use {
            item: "brass key".to_string(),
            target: Some("wooden door".to_string())
        }
    )]
    // "with" alias for the target separator
    #[case::use_with_alias(
        vec!["use", "wrench", "with", "bolt"],
        Action::Use {
            item: "wrench".to_string(),
            target: Some("bolt".to_string())
        }
    )]
    // Single-argument use ("use <item>")
    #[case::use_single_item(
        vec!["use", "potion"],
        Action::Use {
            item: "potion".to_string(),
            target: None
        }
    )]
    // "use" with nothing after it
    #[case::use_alone(vec!["use"], Action::Unknown("use".to_string()))]
    // Fallbacks and malformed inputs
    #[case::unknown_verb(vec!["dance"], Action::Unknown("dance".to_string()))]
    #[case::unknown_direction(vec!["go", "sideways"], Action::Unknown("go sideways".to_string()))]
    #[case::empty_input(vec![], Action::Unknown("".to_string()))]
    fn parses_tokens_into_actions(#[case] tokens: Vec<&str>, #[case] expected: Action) {
        use super::lex;

        assert_eq!(expected, lex(&tokens));
    }
}
