pub mod action;
pub mod direction;
pub mod lexer;
pub mod tokenizer;

pub use action::Action;
pub use direction::Direction;
pub use lexer::lex;
pub use tokenizer::tokenize;

pub fn parse_input(input: &str) -> Action {
    let tokens = tokenize(input);
    let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    lex(&token_refs)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::look("look", Action::Look)]
    #[case::look_shortcut("l", Action::Look)]
    #[case::go_north("go north", Action::Go(Direction::North))]
    #[case::go_east("go east", Action::Go(Direction::East))]
    #[case::go_south("go south", Action::Go(Direction::South))]
    #[case::go_west("go west", Action::Go(Direction::West))]
    #[case::north_shortcut("n", Action::Go(Direction::North))]
    #[case::east_shortcut("e", Action::Go(Direction::East))]
    #[case::south_shortcut("s", Action::Go(Direction::South))]
    #[case::west_shortcut("w", Action::Go(Direction::West))]
    #[case::examine("examine the glowing sword", Action::Examine("glowing sword".to_string()))]
    #[case::examine_shortcut("x chest", Action::Examine("chest".to_string()))]
    #[case::take("take the heavy iron key", Action::Take("heavy iron key".to_string()))]
    #[case::take_shortcut("get lantern", Action::Take("lantern".to_string()))]
    #[case::use_on("use brass key on wooden door",
        Action::Use { item: "brass key".to_string(), target: Some("wooden door".to_string()) })]
    #[case::use_with("use wrench with bolt",
        Action::Use { item: "wrench".to_string(), target: Some("bolt".to_string()) })]
    #[case::use_single("use potion",
        Action::Use { item: "potion".to_string(), target: None })]
    #[case::use_empty("use", Action::Unknown("use".to_string()))]
    #[case::unknown_verb("dance wildly", Action::Unknown("dance wildly".to_string()))]
    #[case::unknown_direction("go sideways", Action::Unknown("go sideways".to_string()))]
    #[case::empty("", Action::Unknown("".to_string()))]
    #[case::mixed_case_punctuation("  EXAMINE the glowing, mysterious sword! ",
        Action::Examine("glowing mysterious sword".to_string()))]
    fn parses_raw_input_into_action(#[case] input: &str, #[case] expected: Action) {
        assert_eq!(expected, parse_input(input));
    }
}
