pub mod action;
pub mod direction;
pub mod parser;
pub mod tokenizer;
pub mod world;

pub use action::Action;
pub use direction::Direction;
pub use parser::lex;
pub use tokenizer::tokenize;
pub use world::{EntityId, Resolution, WorldState};

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::lowercases("take SWord", vec!["take", "sword"])]
    #[case::strips_punctuation("look at the chest!", vec!["look", "chest"])]
    #[case::strips_hyphens("open the brass-key door", vec!["open", "brass", "key", "door"])]
    #[case::collapses_whitespace("examine   the   chest", vec!["examine", "chest"])]
    #[case::drops_stop_words("go to the north in room", vec!["go", "north", "room"])]
    #[case::combined_realistic_sentence(
        "LOOK at tHE gLOwing, mYSTerious sword!",
        vec!["look", "glowing", "mysterious", "sword"]
    )]
    fn tokenizes_and_cleans_input(#[case] input: &str, #[case] expected: Vec<&str>) {
        let tokens = tokenize(input);
        assert_eq!(expected, tokens);
    }

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
    #[case::unknown_direction(vec!["go", "sideways"], Action::Unknown("sideways".to_string()))]
    #[case::empty_input(vec![], Action::Unknown("".to_string()))]
    fn parses_tokens_into_actions(#[case] tokens: Vec<&str>, #[case] expected: Action) {
        assert_eq!(expected, lex(&tokens));
    }

    // Helper fixture setup for world items
    fn sample_world() -> WorldState {
        let mut world = WorldState::new();
        // Adds items with an ID, primary name, and optional aliases
        world.add_item_to_room(
            1,
            "glowing mysterious sword",
            vec!["glowing sword", "sword"],
        );
        world.add_item_to_room(2, "heavy iron key", vec!["iron key", "key"]);
        world.add_item_in_inventory(3, "brass key", vec!["key"]);
        world
    }

    #[rstest]
    #[case::exact_full_name("glowing mysterious sword", Resolution::Found(1))]
    #[case::partial_alias_match("glowing sword", Resolution::Found(1))]
    #[case::alias_match("iron key", Resolution::Found(2))]
    #[case::ambiguous_key("key", Resolution::Ambiguous(vec![2, 3]))]
    #[case::not_found("health potion", Resolution::NotFound)]
    fn resolves_entities_in_world(#[case] target: &str, #[case] expected: Resolution) {
        let world = sample_world();
        let result = world.resolve_entity(target);
        assert_eq!(expected, result);
    }
}
