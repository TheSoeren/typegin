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
