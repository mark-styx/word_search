use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub const WORDS: &[&str] = &[
    "ANCHOVY",
    "AVOCADO",
    "BASIL",
    "BAYLEAF",
    "BEANS",
    "BEEF",
    "BELLPEPPER",
    "BLACKPEPPER",
    "BREADCRUMBS",
    "BROCCOLI",
    "BROTH",
    "BUTTER",
    "CABBAGE",
    "CARROT",
    "CAULIFLOWER",
    "CELERY",
    "CHEDDAR",
    "CHICKEN",
    "CHICKPEAS",
    "CHILIPOWDER",
    "CHIVE",
    "CILANTRO",
    "COCONUTMILK",
    "COLBYJACK",
    "CORN",
    "CORNSTARCH",
    "CREMINI",
    "CUMIN",
    "CURRYPASTE",
    "DATES",
    "DIJON",
    "DILL",
    "EGGPLANT",
    "EGGS",
    "FENNEL",
    "FETA",
    "FISHSAUCE",
    "FLOUR",
    "FONTINA",
    "GARLIC",
    "GINGER",
    "GOATCHEESE",
    "GOCHUJANG",
    "GOUDA",
    "GREEKYOGURT",
    "GREENBEAN",
    "GROUNDBEEF",
    "GRUYERE",
    "HAM",
    "HARISSA",
    "HEAVYCREAM",
    "HOISIN",
    "HONEY",
    "JALAPENO",
    "JASMINERICE",
    "KALE",
    "LASAGNA",
    "LEMON",
    "LETTUCE",
    "LIME",
    "MACARONI",
    "MAPLESYRUP",
    "MAYONNAISE",
    "MINT",
    "MIRIN",
    "MOZZARELLA",
    "MUSHROOM",
    "MUSTARD",
    "NOODLES",
    "NUTMEG",
    "OATS",
    "OLIVEOIL",
    "ONION",
    "OREGANO",
    "PANKO",
    "PAPRIKA",
    "PARMESAN",
    "PARSLEY",
    "PASTA",
    "PEAS",
    "PEPPERJACK",
    "PICKLES",
    "PINEAPPLE",
    "PINTOBEANS",
    "POTATO",
    "POWDERSUGAR",
    "PROVOLONE",
    "QUINOA",
    "RADISH",
    "RAISINS",
    "RAMEN",
    "REDWINEVIN",
    "RICOTTA",
    "RIGATONI",
    "RICE",
    "ROMAINE",
    "ROSEMARY",
    "SAFFRON",
    "SALMON",
    "SALT",
    "SAUERKRAUT",
    "SAUSAGE",
    "SCALLION",
    "SEMOLINA",
    "SESAMEOIL",
    "SESAMESEEDS",
    "SHIITAKE",
    "SHORTRIB",
    "SOYSAUCE",
    "SPAGHETTI",
    "SPINACH",
    "SPLITPEAS",
    "SRIRACHA",
    "STEAK",
    "SUNDRIED",
    "SWEETPOTATO",
    "TAHINI",
    "TAMARI",
    "TEMPEH",
    "THAICHILE",
    "THYME",
    "TOFU",
    "TOMATO",
    "TOMATOPASTE",
    "TOMATOSAUCE",
    "TORTILLA",
    "TUNA",
    "TURMERIC",
    "TURNIP",
    "VANILLA",
    "WORCESTERSHR",
    "YELLOWSQUASH",
    "YOGURT",
    "ZUCCHINI",
];

#[derive(Clone, Copy)]
pub struct RecipeBank {
    pub key: &'static str,
    pub label: &'static str,
    pub words: &'static [&'static str],
}

#[derive(Clone)]
pub struct RecipeSelection {
    pub label: String,
    pub words: Vec<String>,
}

const RECIPE_BANKS: &[RecipeBank] = &[
    RecipeBank {
        key: "chicken_tacos",
        label: "Chicken Tacos",
        words: &[
            "TORTILLA",
            "CHICKEN",
            "ONION",
            "GARLIC",
            "CUMIN",
            "PAPRIKA",
            "OREGANO",
            "LIME",
            "CILANTRO",
            "JALAPENO",
            "TOMATO",
            "AVOCADO",
            "CHEDDAR",
            "SOURCREAM",
            "SALT",
            "OLIVEOIL",
        ],
    },
    RecipeBank {
        key: "beef_chili",
        label: "Beef Chili",
        words: &[
            "GROUNDBEEF",
            "ONION",
            "GARLIC",
            "TOMATO",
            "TOMATOPASTE",
            "BEANS",
            "CUMIN",
            "PAPRIKA",
            "OREGANO",
            "CHILIPOWDER",
            "BROTH",
            "CELERY",
            "BLACKPEPPER",
            "SALT",
            "OLIVEOIL",
        ],
    },
    RecipeBank {
        key: "lasagna",
        label: "Lasagna",
        words: &[
            "LASAGNA",
            "RICOTTA",
            "MOZZARELLA",
            "PARMESAN",
            "GROUNDBEEF",
            "SAUSAGE",
            "ONION",
            "GARLIC",
            "TOMATOSAUCE",
            "OREGANO",
            "BASIL",
            "EGGS",
            "OLIVEOIL",
            "SALT",
            "BLACKPEPPER",
        ],
    },
    RecipeBank {
        key: "fried_rice",
        label: "Fried Rice",
        words: &[
            "RICE",
            "EGGS",
            "PEAS",
            "CARROT",
            "ONION",
            "GARLIC",
            "GINGER",
            "SOYSAUCE",
            "SESAMEOIL",
            "SCALLION",
            "CHICKEN",
            "SHRIMP",
            "CABBAGE",
            "MUSHROOM",
            "THAICHILE",
        ],
    },
    RecipeBank {
        key: "chicken_curry",
        label: "Chicken Curry",
        words: &[
            "CHICKEN",
            "ONION",
            "GARLIC",
            "GINGER",
            "CURRYPASTE",
            "COCONUTMILK",
            "POTATO",
            "CARROT",
            "CILANTRO",
            "LIME",
            "FISHSAUCE",
            "BASIL",
            "JASMINERICE",
            "JALAPENO",
            "SALT",
        ],
    },
    RecipeBank {
        key: "caesar_salad",
        label: "Caesar Salad",
        words: &[
            "ROMAINE",
            "PARMESAN",
            "ANCHOVY",
            "GARLIC",
            "OLIVEOIL",
            "LEMON",
            "MAYONNAISE",
            "DIJON",
            "WORCESTERSHR",
            "BLACKPEPPER",
            "SALT",
            "PANKO",
            "BUTTER",
            "EGGS",
        ],
    },
    RecipeBank {
        key: "beef_stew",
        label: "Beef Stew",
        words: &[
            "BEEF",
            "POTATO",
            "CARROT",
            "ONION",
            "CELERY",
            "GARLIC",
            "TOMATOPASTE",
            "BROTH",
            "FLOUR",
            "THYME",
            "ROSEMARY",
            "BAYLEAF",
            "PEAS",
            "BUTTER",
            "BLACKPEPPER",
            "SALT",
        ],
    },
    RecipeBank {
        key: "omelet",
        label: "Omelet",
        words: &[
            "EGGS",
            "BUTTER",
            "CHEDDAR",
            "PEPPERJACK",
            "SPINACH",
            "MUSHROOM",
            "ONION",
            "TOMATO",
            "CHIVE",
            "SALT",
            "BLACKPEPPER",
            "OLIVEOIL",
            "BASIL",
        ],
    },
    RecipeBank {
        key: "stir_fry",
        label: "Stir Fry",
        words: &[
            "CHICKEN",
            "BROCCOLI",
            "CARROT",
            "BELLPEPPER",
            "ONION",
            "GARLIC",
            "GINGER",
            "SOYSAUCE",
            "SESAMEOIL",
            "RICE",
            "MUSHROOM",
            "CABBAGE",
            "THAICHILE",
            "GREENBEAN",
            "HOISIN",
        ],
    },
    RecipeBank {
        key: "mac_and_cheese",
        label: "Mac and Cheese",
        words: &[
            "MACARONI",
            "CHEDDAR",
            "MONTEREYJACK",
            "COLBYJACK",
            "BUTTER",
            "FLOUR",
            "HEAVYCREAM",
            "MUSTARD",
            "PAPRIKA",
            "SALT",
            "BLACKPEPPER",
            "PANKO",
            "PARMESAN",
        ],
    },
    RecipeBank {
        key: "chicken_noodle_soup",
        label: "Chicken Noodle Soup",
        words: &[
            "CHICKEN",
            "NOODLES",
            "CARROT",
            "CELERY",
            "ONION",
            "GARLIC",
            "BROTH",
            "THYME",
            "PARSLEY",
            "BLACKPEPPER",
            "SALT",
            "BUTTER",
            "LEMON",
        ],
    },
    RecipeBank {
        key: "apple_pie",
        label: "Apple Pie",
        words: &[
            "APPLE",
            "CINNAMON",
            "NUTMEG",
            "ALLSPICE",
            "BUTTER",
            "SUGAR",
            "BROWNSUGAR",
            "FLOUR",
            "PIECRUST",
            "VANILLA",
            "LEMON",
            "SALT",
            "POWDERSUGAR",
        ],
    },
];

fn recent_recipe_cache() -> &'static Mutex<Vec<String>> {
    static RECENT_RECIPES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    RECENT_RECIPES.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn select_recipe_words(count: usize, max_len: Option<usize>) -> Option<RecipeSelection> {
    let mut rng = rand::thread_rng();
    let mut candidates: Vec<&RecipeBank> = RECIPE_BANKS
        .iter()
        .filter(|recipe| {
            recipe
                .words
                .iter()
                .filter(|word| max_len.is_none_or(|max| word.len() <= max))
                .count()
                >= count
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    candidates.shuffle(&mut rng);

    let recent = recent_recipe_cache()
        .lock()
        .ok()
        .map(|cache| cache.clone())
        .unwrap_or_default();
    let recent_positions: HashMap<&str, usize> = recent
        .iter()
        .enumerate()
        .map(|(index, key)| (key.as_str(), index))
        .collect();

    candidates.sort_by_key(|recipe| recent_positions.get(recipe.key).copied().unwrap_or(usize::MAX));
    let recipe = *candidates.last()?;

    let mut words: Vec<String> = recipe
        .words
        .iter()
        .filter(|word| max_len.is_none_or(|max| word.len() <= max))
        .map(|word| (*word).to_string())
        .collect();
    words.shuffle(&mut rng);
    words.truncate(count);

    if let Ok(mut cache) = recent_recipe_cache().lock() {
        cache.push(recipe.key.to_string());
        let cap = RECIPE_BANKS.len().min(6);
        let overflow = cache.len().saturating_sub(cap);
        if overflow > 0 {
            cache.drain(0..overflow);
        }
    }

    Some(RecipeSelection {
        label: recipe.label.to_string(),
        words,
    })
}

#[cfg(test)]
pub fn recipe_labels_containing(words: &[String], max_len: Option<usize>) -> Vec<&'static str> {
    RECIPE_BANKS
        .iter()
        .filter(|recipe| {
            let allowed: std::collections::HashSet<&str> = recipe
                .words
                .iter()
                .filter(|word| max_len.is_none_or(|max| word.len() <= max))
                .copied()
                .collect();
            words.iter().all(|word| allowed.contains(word.as_str()))
        })
        .map(|recipe| recipe.label)
        .collect()
}
