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
    // Lowercasing
    #[case("take SWord", vec!["take", "sword"])]
    // Punctuation acts as a token boundary and gets stripped
    #[case("look at the chest!", vec!["look", "chest"])]
    #[case("open the brass-key door", vec!["open", "brass", "key", "door"])]
    // Whitespace is a token boundary; runs are collapsed
    #[case("examine   the   chest", vec!["examine", "chest"])]
    // Stop words are dropped
    #[case("go to the north in room", vec!["go", "north", "room"])]
    // Combined realistic sentence
    #[case("LOOK at tHE gLOwing, mYSTerious sword!", vec!["look", "glowing", "mysterious", "sword"])]
    fn tokenizes_and_cleans_input(#[case] input: &str, #[case] expected: Vec<&str>) {
        let tokens = tokenize(input);
        assert_eq!(expected, tokens);
    }

    #[rstest]
    // Single-word verb shortcuts
    #[case(vec!["look"], Action::Look)]
    #[case(vec!["l"], Action::Look)]
    // Directional movement & directional shortcuts
    #[case(vec!["go", "north"], Action::Go(Direction::North))]
    #[case(vec!["go", "east"], Action::Go(Direction::East))]
    #[case(vec!["go", "south"], Action::Go(Direction::South))]
    #[case(vec!["go", "west"], Action::Go(Direction::West))]
    #[case(vec!["n"], Action::Go(Direction::North))]
    #[case(vec!["e"], Action::Go(Direction::East))]
    #[case(vec!["s"], Action::Go(Direction::South))]
    #[case(vec!["w"], Action::Go(Direction::West))]
    #[case(vec!["north"], Action::Go(Direction::North))]
    #[case(vec!["east"], Action::Go(Direction::East))]
    #[case(vec!["south"], Action::Go(Direction::South))]
    #[case(vec!["west"], Action::Go(Direction::West))]
    // Verb + Noun targets (re-joining descriptors)
    #[case(vec!["examine", "glowing", "sword"], Action::Examine("glowing sword".to_string()))]
    #[case(vec!["x", "chest"], Action::Examine("chest".to_string()))]
    #[case(vec!["take", "heavy", "iron", "key"], Action::Take("heavy iron key".to_string()))]
    #[case(vec!["get", "lantern"], Action::Take("lantern".to_string()))]
    // Two-argument interaction ("use <item> on <target>")
    #[case(
        vec!["use", "brass", "key", "on", "wooden", "door"],
        Action::Use {
            item: "brass key".to_string(),
            target: Some("wooden door".to_string())
        }
    )]
    // "with" alias for the target separator
    #[case(
        vec!["use", "wrench", "with", "bolt"],
        Action::Use {
            item: "wrench".to_string(),
            target: Some("bolt".to_string())
        }
    )]
    // Single-argument use ("use <item>")
    #[case(
        vec!["use", "potion"],
        Action::Use {
            item: "potion".to_string(),
            target: None
        }
    )]
    // "use" with nothing after it
    #[case(vec!["use"], Action::Unknown("use".to_string()))]
    // Fallbacks and malformed inputs
    #[case(vec!["dance"], Action::Unknown("dance".to_string()))]
    #[case(vec!["go", "sideways"], Action::Unknown("sideways".to_string()))]
    #[case(vec![], Action::Unknown("".to_string()))]
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
    // 1. Exact match by full name
    #[case("glowing mysterious sword", Resolution::Found(1))]
    // 2. Partial / alias match
    #[case("glowing sword", Resolution::Found(1))]
    #[case("iron key", Resolution::Found(2))]
    // 3. Ambiguous match: "key" matches both "heavy iron key" in room (2) and "brass key" in inventory (3)
    #[case("key", Resolution::Ambiguous(vec![2, 3]))]
    // 4. Missing target
    #[case("health potion", Resolution::NotFound)]
    fn resolves_entities_in_world(#[case] target: &str, #[case] expected: Resolution) {
        let world = sample_world();
        let result = world.resolve_entity(target);
        assert_eq!(expected, result);
    }
}
