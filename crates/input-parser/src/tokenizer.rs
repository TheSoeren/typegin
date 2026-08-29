const STOP_WORDS: &[&str] = &["a", "an", "the", "at", "to", "in", "from", "of", "about"];

fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(&word)
}

pub fn tokenize(input: &str) -> Vec<String> {
    input
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|token| !token.is_empty())
        .filter(|token| !is_stop_word(token))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
}
