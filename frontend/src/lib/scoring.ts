import type { Difficulty } from "@/types";

const DIFFICULTY_MULTIPLIERS: Record<Difficulty, number> = {
  easy: 1,
  medium: 2,
  hard: 3,
  expert: 5,
};

export function scoreWord(word: string, difficulty: Difficulty) {
  const shortWordBonus = Math.floor((Math.max(12 - word.length, 0) + 1) / 2) + 2;
  return shortWordBonus * DIFFICULTY_MULTIPLIERS[difficulty];
}
