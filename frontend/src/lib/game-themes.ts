import type { Difficulty, WordTheme } from "@/types";

export const themeLabels: Record<WordTheme, string> = {
  animals: "Animals",
  cats: "Cats",
  boating: "Boating",
  camping: "Camping",
  recipe_ingredients: "Recipe Ingredients",
  sports: "Sports",
  food: "Food",
  fishing: "Fishing",
  pizza: "Pizza",
  snacks: "Snacks",
  fast_food: "Fast Food",
  bugs: "Bugs",
  movies: "Disney",
  cars: "Cars",
  desert: "Desert",
  mountains: "Mountains",
  ocean: "Ocean",
  weather: "Weather",
  garden: "Garden",
  farm: "Farm",
  shiny_things: "Shiny Things",
  toys: "Toys",
  space: "Space",
  science: "Science",
  geography: "Geography",
  technology: "Technology",
  music: "Music",
  history: "History",
  home_renovation: "Home Renovation",
  luxury_brands: "Luxury Brands",
  nature: "Nature",
};

export const difficultyLabels: Record<Difficulty, string> = {
  easy: "Easy (8x8)",
  medium: "Medium (12x12)",
  hard: "Hard (15x15)",
  expert: "Expert (20x20)",
};

export const themeOptions = Object.entries(themeLabels).map(([value, label]) => ({
  value,
  label,
}));

export const difficultyOptions = Object.entries(difficultyLabels).map(([value, label]) => ({
  value,
  label,
}));

export function getThemeLabel(value: string) {
  if (value in themeLabels) {
    return themeLabels[value as WordTheme];
  }

  const normalized = value.trim().toLowerCase().replace(/\s+/g, "_");
  if (normalized in themeLabels) {
    return themeLabels[normalized as WordTheme];
  }

  return value;
}
