use crate::models::WordTheme;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy)]
pub struct BuiltInWordBank {
    pub key: &'static str,
    pub label: &'static str,
    pub theme: WordTheme,
}

const BUILT_IN_WORD_BANKS: &[BuiltInWordBank] = &[
    BuiltInWordBank {
        key: "animals",
        label: "Animals",
        theme: WordTheme::Animals,
    },
    BuiltInWordBank {
        key: "cats",
        label: "Cats",
        theme: WordTheme::Cats,
    },
    BuiltInWordBank {
        key: "boating",
        label: "Boating",
        theme: WordTheme::Boating,
    },
    BuiltInWordBank {
        key: "camping",
        label: "Camping",
        theme: WordTheme::Camping,
    },
    BuiltInWordBank {
        key: "recipe_ingredients",
        label: "Recipe Ingredients",
        theme: WordTheme::RecipeIngredients,
    },
    BuiltInWordBank {
        key: "sports",
        label: "Sports",
        theme: WordTheme::Sports,
    },
    BuiltInWordBank {
        key: "food",
        label: "Food",
        theme: WordTheme::Food,
    },
    BuiltInWordBank {
        key: "fishing",
        label: "Fishing",
        theme: WordTheme::Fishing,
    },
    BuiltInWordBank {
        key: "pizza",
        label: "Pizza",
        theme: WordTheme::Pizza,
    },
    BuiltInWordBank {
        key: "snacks",
        label: "Snacks",
        theme: WordTheme::Snacks,
    },
    BuiltInWordBank {
        key: "fast_food",
        label: "Fast Food",
        theme: WordTheme::FastFood,
    },
    BuiltInWordBank {
        key: "bugs",
        label: "Bugs",
        theme: WordTheme::Bugs,
    },
    BuiltInWordBank {
        key: "movies",
        label: "Disney",
        theme: WordTheme::Movies,
    },
    BuiltInWordBank {
        key: "cars",
        label: "Cars",
        theme: WordTheme::Cars,
    },
    BuiltInWordBank {
        key: "desert",
        label: "Desert",
        theme: WordTheme::Desert,
    },
    BuiltInWordBank {
        key: "mountains",
        label: "Mountains",
        theme: WordTheme::Mountains,
    },
    BuiltInWordBank {
        key: "ocean",
        label: "Ocean",
        theme: WordTheme::Ocean,
    },
    BuiltInWordBank {
        key: "weather",
        label: "Weather",
        theme: WordTheme::Weather,
    },
    BuiltInWordBank {
        key: "garden",
        label: "Garden",
        theme: WordTheme::Garden,
    },
    BuiltInWordBank {
        key: "farm",
        label: "Farm",
        theme: WordTheme::Farm,
    },
    BuiltInWordBank {
        key: "shiny_things",
        label: "Shiny Things",
        theme: WordTheme::ShinyThings,
    },
    BuiltInWordBank {
        key: "toys",
        label: "Toys",
        theme: WordTheme::Toys,
    },
    BuiltInWordBank {
        key: "space",
        label: "Space",
        theme: WordTheme::Space,
    },
    BuiltInWordBank {
        key: "science",
        label: "Science",
        theme: WordTheme::Science,
    },
    BuiltInWordBank {
        key: "geography",
        label: "Geography",
        theme: WordTheme::Geography,
    },
    BuiltInWordBank {
        key: "technology",
        label: "Technology",
        theme: WordTheme::Technology,
    },
    BuiltInWordBank {
        key: "music",
        label: "Music",
        theme: WordTheme::Music,
    },
    BuiltInWordBank {
        key: "history",
        label: "History",
        theme: WordTheme::History,
    },
    BuiltInWordBank {
        key: "home_renovation",
        label: "Home Renovation",
        theme: WordTheme::HomeRenovation,
    },
    BuiltInWordBank {
        key: "luxury_brands",
        label: "Luxury Brands",
        theme: WordTheme::LuxuryBrands,
    },
    BuiltInWordBank {
        key: "nature",
        label: "Nature",
        theme: WordTheme::Nature,
    },
];

pub fn built_in_word_banks() -> &'static [BuiltInWordBank] {
    BUILT_IN_WORD_BANKS
}

pub fn parse_theme_key(value: &str) -> Option<WordTheme> {
    BUILT_IN_WORD_BANKS
        .iter()
        .find(|entry| entry.key == value)
        .map(|entry| entry.theme)
}

pub fn theme_key(theme: WordTheme) -> &'static str {
    BUILT_IN_WORD_BANKS
        .iter()
        .find(|entry| entry.theme == theme)
        .map(|entry| entry.key)
        .unwrap_or("science")
}

pub fn theme_label(theme: WordTheme) -> &'static str {
    BUILT_IN_WORD_BANKS
        .iter()
        .find(|entry| entry.theme == theme)
        .map(|entry| entry.label)
        .unwrap_or("Science")
}

pub fn get_all_words(theme: WordTheme) -> &'static [&'static str] {
    match theme {
        WordTheme::Animals => super::animals::WORDS,
        WordTheme::Cats => super::cats::WORDS,
        WordTheme::Boating => super::boating::WORDS,
        WordTheme::Camping => super::camping::WORDS,
        WordTheme::RecipeIngredients => super::recipe_ingredients::WORDS,
        WordTheme::Sports => super::sports::WORDS,
        WordTheme::Food => super::food::WORDS,
        WordTheme::Fishing => super::fishing::WORDS,
        WordTheme::Pizza => super::pizza::WORDS,
        WordTheme::Snacks => super::snacks::WORDS,
        WordTheme::FastFood => super::fast_food::WORDS,
        WordTheme::Bugs => super::bugs::WORDS,
        WordTheme::Movies => super::movies::WORDS,
        WordTheme::Cars => super::cars::WORDS,
        WordTheme::Desert => super::desert::WORDS,
        WordTheme::Mountains => super::mountains::WORDS,
        WordTheme::Ocean => super::ocean::WORDS,
        WordTheme::Weather => super::weather::WORDS,
        WordTheme::Garden => super::garden::WORDS,
        WordTheme::Farm => super::farm::WORDS,
        WordTheme::ShinyThings => super::shiny_things::WORDS,
        WordTheme::Toys => super::toys::WORDS,
        WordTheme::Space => super::space::WORDS,
        WordTheme::Science => super::science::WORDS,
        WordTheme::Geography => super::geography::WORDS,
        WordTheme::Technology => super::technology::WORDS,
        WordTheme::Music => super::music::WORDS,
        WordTheme::History => super::history::WORDS,
        WordTheme::HomeRenovation => super::home_renovation::WORDS,
        WordTheme::LuxuryBrands => super::luxury_brands::WORDS,
        WordTheme::Nature => super::nature::WORDS,
    }
}

fn recent_word_cache() -> &'static Mutex<HashMap<String, Vec<String>>> {
    static RECENT_WORDS: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    RECENT_WORDS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn get_words(theme: WordTheme, count: usize, max_len: Option<usize>) -> Vec<String> {
    if theme == WordTheme::RecipeIngredients {
        return super::recipe_ingredients::select_recipe_words(count, max_len)
            .map(|selection| selection.words)
            .unwrap_or_default();
    }

    let all: Vec<String> = get_all_words(theme)
        .iter()
        .map(|word| (*word).to_string())
        .collect();
    select_words(
        &format!("builtin:{}", theme_key(theme)),
        &all,
        count,
        max_len,
    )
}

pub fn get_custom_words(
    bank_id: &str,
    all_words: &[String],
    count: usize,
    max_len: Option<usize>,
) -> Vec<String> {
    select_words(&format!("custom:{bank_id}"), all_words, count, max_len)
}

fn select_words(
    cache_key: &str,
    all_words: &[String],
    count: usize,
    max_len: Option<usize>,
) -> Vec<String> {
    let mut rng = rand::thread_rng();
    let mut candidates: Vec<&str> = all_words
        .iter()
        .map(String::as_str)
        .filter(|word| max_len.is_none_or(|max| word.len() <= max))
        .collect();
    candidates.shuffle(&mut rng);

    let recent = recent_word_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(cache_key).cloned())
        .unwrap_or_default();
    let recent_positions: HashMap<&str, usize> = recent
        .iter()
        .enumerate()
        .map(|(index, word)| (word.as_str(), index))
        .collect();

    let mut fresh = Vec::new();
    let mut stale = Vec::new();
    for word in candidates {
        if let Some(position) = recent_positions.get(word) {
            stale.push((*position, word));
        } else {
            fresh.push(word);
        }
    }
    stale.sort_by_key(|(position, _)| *position);

    let selected: Vec<String> = fresh
        .into_iter()
        .chain(stale.into_iter().map(|(_, word)| word))
        .take(count)
        .map(str::to_string)
        .collect();

    if let Ok(mut cache) = recent_word_cache().lock() {
        let history = cache.entry(cache_key.to_string()).or_default();
        for word in &selected {
            if let Some(index) = history.iter().position(|existing| existing == word) {
                history.remove(index);
            }
        }
        history.extend(selected.iter().cloned());

        let history_cap = all_words
            .len()
            .saturating_sub(selected.len())
            .max(selected.len());
        let overflow = history.len().saturating_sub(history_cap);
        if overflow > 0 {
            history.drain(0..overflow);
        }
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::puzzles::recipe_ingredients;

    fn all_themes() -> Vec<WordTheme> {
        vec![
            WordTheme::Animals,
            WordTheme::Cats,
            WordTheme::Boating,
            WordTheme::Camping,
            WordTheme::RecipeIngredients,
            WordTheme::Sports,
            WordTheme::Food,
            WordTheme::Fishing,
            WordTheme::Pizza,
            WordTheme::Snacks,
            WordTheme::FastFood,
            WordTheme::Bugs,
            WordTheme::Movies,
            WordTheme::Cars,
            WordTheme::Desert,
            WordTheme::Mountains,
            WordTheme::Ocean,
            WordTheme::Weather,
            WordTheme::Garden,
            WordTheme::Farm,
            WordTheme::ShinyThings,
            WordTheme::Toys,
            WordTheme::Space,
            WordTheme::Science,
            WordTheme::Geography,
            WordTheme::Technology,
            WordTheme::Music,
            WordTheme::History,
            WordTheme::HomeRenovation,
            WordTheme::LuxuryBrands,
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
            let result = get_words(theme, 10, None);
            assert_eq!(result.len(), 10, "Theme {:?}", theme);
        }
    }

    #[test]
    fn get_words_no_duplicates() {
        let result = get_words(WordTheme::Animals, 20, None);
        let set: std::collections::HashSet<_> = result.iter().collect();
        assert_eq!(set.len(), 20);
    }

    #[test]
    fn get_words_respects_max_len() {
        let result = get_words(WordTheme::Animals, 6, Some(8));
        for word in &result {
            assert!(
                word.len() <= 8,
                "Word '{}' has length {} which exceeds max_len 8",
                word,
                word.len()
            );
        }
    }

    #[test]
    fn get_words_avoids_immediate_repeats_when_theme_has_headroom() {
        let first = get_words(WordTheme::Science, 20, None);
        let second = get_words(WordTheme::Science, 20, None);
        let overlap: std::collections::HashSet<_> = first.iter().collect();
        assert!(
            second.iter().all(|word| !overlap.contains(word)),
            "recent-word rotation should avoid immediate repeats"
        );
    }

    #[test]
    fn recipe_ingredients_selects_a_single_recipe() {
        let result = get_words(WordTheme::RecipeIngredients, 13, Some(12));
        assert_eq!(result.len(), 13);

        let matches = recipe_ingredients::recipe_labels_containing(&result, Some(12));
        assert!(
            !matches.is_empty(),
            "selected ingredients should fit at least one recipe, got {:?}",
            result
        );
    }
}
