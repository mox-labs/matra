//! Readability metric — Flesch-Kincaid grade level per paragraph.

use crate::domain::Document;

/// Populate `Paragraph::readability_grade` for every paragraph with
/// more than 10 whitespace-counted words that is not in a blockquote.
pub fn compute(analysis: &mut Document) {
    for para in analysis.paragraphs_mut() {
        if para.word_count() > 10 && !para.in_blockquote {
            para.readability_grade = Some(flesch_kincaid_grade(&para.text));
        }
    }
}

/// Flesch-Kincaid grade level. Uses raw whitespace splitting per the
/// formula's original specification, not NLP token counts.
pub fn flesch_kincaid_grade(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().collect();
    let n = words.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let sents = text
        .chars()
        .filter(|c| matches!(c, '.' | '!' | '?'))
        .count()
        .max(1) as f64;
    let syllables: f64 = words
        .iter()
        .map(|w| {
            let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();
            if clean.is_empty() {
                0.0
            } else {
                syllable_count(&clean) as f64
            }
        })
        .sum();
    0.39 * (n / sents) + 11.8 * (syllables / n) - 15.59
}

fn syllable_count(word: &str) -> usize {
    let word = word.to_lowercase();
    if word.len() <= 3 {
        return 1;
    }
    let vowels = b"aeiouy";
    let mut count = 0;
    let mut prev_vowel = false;
    for &b in word.as_bytes() {
        let is_vowel = vowels.contains(&b);
        if is_vowel && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_vowel;
    }
    if word.ends_with('e') && count > 1 {
        count -= 1;
    }
    count.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syllable_count_basic() {
        assert_eq!(syllable_count("the"), 1);
        assert_eq!(syllable_count("beautiful"), 3);
    }

    #[test]
    fn flesch_kincaid_in_range() {
        let grade = flesch_kincaid_grade("The cat sat on the mat. The dog chased the cat.");
        assert!(grade > -5.0 && grade < 20.0, "FK grade was {grade}");
    }
}
