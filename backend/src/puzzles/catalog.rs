use crate::models::WordTheme;
use rand::seq::SliceRandom;

pub fn get_all_words(theme: WordTheme) -> &'static [&'static str] {
    match theme {
        WordTheme::Animals => super::animals::WORDS,
        WordTheme::Sports => super::sports::WORDS,
        WordTheme::Food => super::food::WORDS,
        WordTheme::Movies => super::movies::WORDS,
        WordTheme::Science => super::science::WORDS,
        WordTheme::Geography => super::geography::WORDS,
        WordTheme::Technology => super::technology::WORDS,
        WordTheme::Music => super::music::WORDS,
        WordTheme::History => super::history::WORDS,
        WordTheme::Nature => super::nature::WORDS,
    }
}

pub fn get_words(theme: WordTheme, count: usize) -> Vec<String> {
    let all = get_all_words(theme);
    let mut rng = rand::thread_rng();
    let mut indices: Vec<usize> = (0..all.len()).collect();
    indices.shuffle(&mut rng);
    indices
        .into_iter()
        .take(count)
        .map(|i| all[i].to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_themes() -> Vec<WordTheme> {
        vec![
            WordTheme::Animals,
            WordTheme::Sports,
            WordTheme::Food,
            WordTheme::Movies,
            WordTheme::Science,
            WordTheme::Geography,
            WordTheme::Technology,
            WordTheme::Music,
            WordTheme::History,
            WordTheme::Nature,
        ]
    }

    #[test]
    fn each_theme_has_at_least_60_words() {
        for theme in all_themes() {
            let words = get_all_words(theme);
            assert!(
                words.len() >= 60,
                "Theme {:?} has only {} words, need >= 60",
                theme,
                words.len()
            );
        }
    }

    #[test]
    fn all_words_are_valid() {
        for theme in all_themes() {
            for word in get_all_words(theme) {
                assert!(
                    (2..=15).contains(&word.len()),
                    "Word '{}' in {:?} has length {} (need 2-15)",
                    word,
                    theme,
                    word.len()
                );
                assert!(
                    word.chars().all(|c| c.is_ascii_uppercase()),
                    "Word '{}' in {:?} contains non-uppercase-ASCII",
                    word,
                    theme
                );
            }
        }
    }

    #[test]
    fn no_duplicates_within_theme() {
        for theme in all_themes() {
            let words = get_all_words(theme);
            let mut seen = std::collections::HashSet::new();
            for word in words {
                assert!(
                    seen.insert(word),
                    "Duplicate word '{}' in theme {:?}",
                    word,
                    theme
                );
            }
        }
    }

    #[test]
    fn get_words_returns_correct_count() {
        for theme in all_themes() {
            let result = get_words(theme, 10);
            assert_eq!(result.len(), 10, "Theme {:?}", theme);
        }
    }

    #[test]
    fn get_words_no_duplicates() {
        let result = get_words(WordTheme::Animals, 20);
        let set: std::collections::HashSet<_> = result.iter().collect();
        assert_eq!(set.len(), 20);
    }
}
