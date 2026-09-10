const STOP_WORDS: &[&str] = &["a", "an", "the", "at", "to", "in", "from", "of", "about"];

fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(&word)
}

/// Split `input` into lowercase word tokens, dropping punctuation and stop
/// words.
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
    use super::*;

    #[test]
    fn splits_on_whitespace() {
        assert_eq!(tokenize("take sword"), vec!["take", "sword"]);
    }

    #[test]
    fn lowercases_words() {
        assert_eq!(tokenize("TAKE Sword"), vec!["take", "sword"]);
    }

    #[test]
    fn drops_punctuation() {
        assert_eq!(
            tokenize("take the sword, please!"),
            vec!["take", "sword", "please"]
        );
    }

    #[test]
    fn drops_stop_words() {
        for word in ["a", "an", "the", "at", "to", "in", "from", "of", "about"] {
            assert!(
                tokenize(word).is_empty(),
                "expected {word:?} to be dropped as a stop word"
            );
        }
    }

    #[test]
    fn stop_word_inside_a_longer_phrase_is_dropped() {
        assert_eq!(
            tokenize("ask about the exit"),
            vec!["ask", "exit"],
            "stop words inside a phrase should be stripped, not just at the edges"
        );
    }

    #[test]
    fn collapses_repeated_whitespace() {
        assert_eq!(tokenize("take   the   sword"), vec!["take", "sword"]);
    }

    #[test]
    fn leading_and_trailing_whitespace_is_ignored() {
        assert_eq!(tokenize("  take sword  "), vec!["take", "sword"]);
    }

    #[test]
    fn empty_input_yields_no_tokens() {
        assert!(tokenize("").is_empty());
    }

    #[test]
    fn input_of_only_stop_words_yields_no_tokens() {
        assert!(tokenize("the a an").is_empty());
    }

    #[test]
    fn hyphens_split_into_separate_tokens() {
        assert_eq!(
            tokenize("brass-key"),
            vec!["brass", "key"],
            "ascii punctuation, including hyphens, is a token boundary"
        );
    }

    #[test]
    fn is_stop_word_matches_known_words_only() {
        assert!(is_stop_word("the"));
        assert!(!is_stop_word("sword"));
    }
}
