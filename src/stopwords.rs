//! Shared stop word list. Used by encoders (lexical density) and extraction algorithms.
//!
//! Sorted alphabetically for binary search. If you add words, keep the order.

/// Sorted stop word list. Use [`is_stop_word`] for lookups.
pub(crate) const STOP_WORDS: &[&str] = &[
    "a", "above", "after", "again", "all", "an", "and", "are", "as", "at", "be", "been", "before",
    "being", "below", "between", "both", "but", "by", "can", "could", "did", "do", "does",
    "during", "each", "few", "for", "from", "had", "has", "have", "he", "her", "him", "his", "how",
    "i", "if", "in", "into", "is", "it", "its", "just", "may", "me", "might", "more", "most", "my",
    "no", "nor", "not", "now", "of", "off", "on", "once", "only", "or", "other", "our", "out",
    "over", "own", "same", "shall", "she", "should", "so", "some", "such", "than", "that", "the",
    "their", "them", "then", "there", "these", "they", "this", "those", "through", "to", "too",
    "under", "very", "was", "we", "were", "what", "when", "where", "which", "while", "who", "whom",
    "why", "will", "with", "would", "you", "your",
];

/// O(log n) stop word check via binary search on sorted slice.
pub(crate) fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.binary_search(&word).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_words_are_sorted() {
        for pair in STOP_WORDS.windows(2) {
            assert!(
                pair[0] < pair[1],
                "STOP_WORDS not sorted: {:?} >= {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn is_stop_word_finds_words() {
        assert!(is_stop_word("the"));
        assert!(is_stop_word("a"));
        assert!(is_stop_word("your"));
        assert!(!is_stop_word("architecture"));
        assert!(!is_stop_word("rust"));
    }
}
