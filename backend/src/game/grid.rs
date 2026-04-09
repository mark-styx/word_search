use crate::models::{Difficulty, Grid, PlacedWord, WordDirection};
use rand::Rng;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum GenerationError {
    #[error("word '{word}' could not be placed after {attempts} attempts")]
    PlacementFailed { word: String, attempts: usize },
    #[error("word list is empty")]
    EmptyWordList,
    #[error("word '{word}' is longer than grid size {grid_size}")]
    WordTooLong { word: String, grid_size: usize },
}

const EMPTY_CELL: char = '\0';
const MAX_PLACEMENT_ATTEMPTS: usize = 100;

/// Generate a word search grid for the given words and difficulty.
pub fn generate_grid(words: &[String], difficulty: Difficulty) -> Result<Grid, GenerationError> {
    if words.is_empty() {
        return Err(GenerationError::EmptyWordList);
    }

    let size = difficulty.grid_size();

    // Filter out words that are too long
    for word in words {
        if word.len() > size {
            return Err(GenerationError::WordTooLong {
                word: word.clone(),
                grid_size: size,
            });
        }
    }

    // Create empty grid
    let mut cells = vec![vec![EMPTY_CELL; size]; size];
    let mut placed_words: Vec<PlacedWord> = Vec::new();
    let directions = difficulty.allowed_directions();
    let mut rng = rand::thread_rng();

    // Sort words longest-first for better placement success
    let mut sorted_words: Vec<&String> = words.iter().collect();
    sorted_words.sort_by(|a, b| b.len().cmp(&a.len()));

    for word in sorted_words {
        let upper = word.to_uppercase();
        let mut placed = false;

        for _ in 0..MAX_PLACEMENT_ATTEMPTS {
            let dir = directions[rng.gen_range(0..directions.len())];
            let start_row = rng.gen_range(0..size);
            let start_col = rng.gen_range(0..size);

            if let Some(pw) = try_place_word(&upper, start_row, start_col, dir, &cells, size) {
                // Place the word onto the grid
                let (dr, dc) = dir.delta();
                for (i, ch) in upper.chars().enumerate() {
                    let r = (start_row as isize + dr * i as isize) as usize;
                    let c = (start_col as isize + dc * i as isize) as usize;
                    cells[r][c] = ch;
                }
                placed_words.push(pw);
                placed = true;
                break;
            }
        }

        if !placed {
            return Err(GenerationError::PlacementFailed {
                word: word.clone(),
                attempts: MAX_PLACEMENT_ATTEMPTS,
            });
        }
    }

    // Fill remaining empty cells with random uppercase letters
    for row in &mut cells {
        for cell in row {
            if *cell == EMPTY_CELL {
                *cell = rng.gen_range(b'A'..=b'Z') as char;
            }
        }
    }

    Ok(Grid {
        cells,
        width: size,
        height: size,
        placed_words,
    })
}

/// Try to place a word at the given position and direction.
/// Returns Some(PlacedWord) if placement succeeds, None otherwise.
fn try_place_word(
    word: &str,
    start_row: usize,
    start_col: usize,
    direction: WordDirection,
    cells: &[Vec<char>],
    grid_size: usize,
) -> Option<PlacedWord> {
    let (dr, dc) = direction.delta();
    let len = word.chars().count();
    if len == 0 {
        return None;
    }

    // Check that the end position is within bounds
    let end_row = start_row as isize + dr * (len as isize - 1);
    let end_col = start_col as isize + dc * (len as isize - 1);

    if end_row < 0
        || end_row >= grid_size as isize
        || end_col < 0
        || end_col >= grid_size as isize
    {
        return None;
    }

    // Validate each cell along the path
    let mut current_row = start_row as isize;
    let mut current_col = start_col as isize;
    for ch in word.chars() {
        let r = current_row as usize;
        let c = current_col as usize;
        let existing = cells[r][c];
        // Cell must be empty or contain the same letter
        if existing != EMPTY_CELL && existing != ch {
            return None;
        }
        current_row += dr;
        current_col += dc;
    }

    Some(PlacedWord {
        word: word.to_string(),
        start: (start_row, start_col),
        end: (end_row as usize, end_col as usize),
        direction,
    })
}

/// Validate whether a selection of cells forms a valid straight line matching a placed word.
pub fn validate_selection(grid: &Grid, cells: Vec<(usize, usize)>) -> Option<String> {
    if cells.len() < 2 {
        return None;
    }

    // Check that all cells form a straight line with uniform step
    let dr = cells[1].0 as isize - cells[0].0 as isize;
    let dc = cells[1].1 as isize - cells[0].1 as isize;
    if dr == 0 && dc == 0 {
        return None; // duplicate cell
    }

    for i in 1..cells.len() {
        let expected_r = cells[0].0 as isize + dr * i as isize;
        let expected_c = cells[0].1 as isize + dc * i as isize;
        if cells[i].0 as isize != expected_r || cells[i].1 as isize != expected_c {
            return None;
        }
    }

    // Build the string from selected cells
    let selected_word: String = cells.iter().map(|&(r, c)| grid.cells[r][c]).collect();

    // Check against all placed words (both forward and reverse)
    for pw in &grid.placed_words {
        if pw.word == selected_word {
            return Some(pw.word.clone());
        }
        let reversed: String = pw.word.chars().rev().collect();
        if reversed == selected_word {
            return Some(pw.word.clone());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper ──────────────────────────────────────────────────────────

    /// Build a grid from a 2-D char array for deterministic tests.
    fn grid_from_rows(rows: &[&str]) -> Grid {
        let cells: Vec<Vec<char>> = rows.iter().map(|r| r.chars().collect()).collect();
        let height = cells.len();
        let width = cells[0].len();
        Grid {
            cells,
            width,
            height,
            placed_words: Vec::new(),
        }
    }

    // ── GenerationError unit tests ──────────────────────────────────────

    #[test]
    fn empty_word_list_returns_error() {
        let err = generate_grid(&[], Difficulty::Easy).unwrap_err();
        assert_eq!(err, GenerationError::EmptyWordList);
    }

    #[test]
    fn word_too_long_returns_error() {
        let words = vec!["ABCDEFGHIJKLMNO".to_string()]; // 15 chars, easy grid is 8
        let err = generate_grid(&words, Difficulty::Easy).unwrap_err();
        assert_eq!(
            err,
            GenerationError::WordTooLong {
                word: "ABCDEFGHIJKLMNO".to_string(),
                grid_size: 8,
            }
        );
    }

    // ── Grid generation – all difficulties ──────────────────────────────

    #[test]
    fn generate_easy_grid() {
        let words = vec![
            "CAT".to_string(),
            "DOG".to_string(),
            "SUN".to_string(),
            "HAT".to_string(),
            "RUN".to_string(),
            "BAT".to_string(),
        ];
        let grid = generate_grid(&words, Difficulty::Easy).unwrap();
        assert_eq!(grid.width, 8);
        assert_eq!(grid.height, 8);
        assert_eq!(grid.placed_words.len(), 6);
        assert_no_empty_cells(&grid);
        assert_all_words_placed(&grid, &words);
    }

    #[test]
    fn generate_medium_grid() {
        let words = vec![
            "PYTHON".to_string(),
            "RUST".to_string(),
            "JAVA".to_string(),
            "CODE".to_string(),
            "SWIFT".to_string(),
            "DEBUG".to_string(),
            "STACK".to_string(),
            "QUEUE".to_string(),
            "ARRAY".to_string(),
            "LOOP".to_string(),
        ];
        let grid = generate_grid(&words, Difficulty::Medium).unwrap();
        assert_eq!(grid.width, 12);
        assert_eq!(grid.height, 12);
        assert_eq!(grid.placed_words.len(), 10);
        assert_no_empty_cells(&grid);
        assert_all_words_placed(&grid, &words);
    }

    #[test]
    fn generate_hard_grid() {
        let words: Vec<String> = (0..15).map(|i| format!("WORD{}", i)).collect();
        let grid = generate_grid(&words, Difficulty::Hard).unwrap();
        assert_eq!(grid.width, 15);
        assert_eq!(grid.height, 15);
        assert_eq!(grid.placed_words.len(), 15);
        assert_no_empty_cells(&grid);
        assert_all_words_placed(&grid, &words);
    }

    #[test]
    fn generate_expert_grid() {
        let words: Vec<String> = (0..20).map(|i| format!("TEST{}", i)).collect();
        let grid = generate_grid(&words, Difficulty::Expert).unwrap();
        assert_eq!(grid.width, 20);
        assert_eq!(grid.height, 20);
        assert_eq!(grid.placed_words.len(), 20);
        assert_no_empty_cells(&grid);
        assert_all_words_placed(&grid, &words);
    }

    // ── Word placement correctness ──────────────────────────────────────

    #[test]
    fn placed_words_are_within_bounds() {
        let words = vec!["CAT".to_string(), "DOG".to_string()];
        let grid = generate_grid(&words, Difficulty::Easy).unwrap();
        for pw in &grid.placed_words {
            assert!(pw.start.0 < grid.height);
            assert!(pw.start.1 < grid.width);
            assert!(pw.end.0 < grid.height);
            assert!(pw.end.1 < grid.width);
        }
    }

    #[test]
    fn placed_words_match_cells() {
        let words = vec!["HELLO".to_string(), "WORLD".to_string()];
        let grid = generate_grid(&words, Difficulty::Easy).unwrap();
        for pw in &grid.placed_words {
            let (dr, dc) = pw.direction.delta();
            let chars: String = (0..pw.word.len())
                .map(|i| {
                    let r = pw.start.0 as isize + dr * i as isize;
                    let c = pw.start.1 as isize + dc * i as isize;
                    grid.cells[r as usize][c as usize]
                })
                .collect();
            assert_eq!(chars, pw.word);
        }
    }

    #[test]
    fn easy_uses_only_horizontal_and_vertical() {
        let words = vec!["CAT".to_string(), "DOG".to_string(), "SUN".to_string()];
        let grid = generate_grid(&words, Difficulty::Easy).unwrap();
        for pw in &grid.placed_words {
            assert!(
                pw.direction == WordDirection::Horizontal
                    || pw.direction == WordDirection::Vertical,
                "Easy should only use horizontal/vertical, got {:?}",
                pw.direction
            );
        }
    }

    // ── Collision detection ─────────────────────────────────────────────

    #[test]
    fn same_letter_overlap_is_allowed() {
        let words = vec!["CAT".to_string(), "CAB".to_string()];
        let grid = generate_grid(&words, Difficulty::Easy).unwrap();
        assert_eq!(grid.placed_words.len(), 2);
    }

    #[test]
    fn different_letter_collision_is_rejected() {
        let size = 5;
        let mut cells = vec![vec![EMPTY_CELL; size]; size];
        cells[2][2] = 'X';

        let result = try_place_word("ABC", 2, 1, WordDirection::Horizontal, &cells, size);
        assert!(
            result.is_none(),
            "Should reject placement where 'B' conflicts with 'X'"
        );
    }

    #[test]
    fn same_letter_collision_is_accepted() {
        let size = 5;
        let mut cells = vec![vec![EMPTY_CELL; size]; size];
        cells[2][2] = 'B';

        let result = try_place_word("ABC", 2, 1, WordDirection::Horizontal, &cells, size);
        assert!(
            result.is_some(),
            "Should accept placement where letters match"
        );
    }

    #[test]
    fn out_of_bounds_placement_is_rejected() {
        let cells = vec![vec![EMPTY_CELL; 5]; 5];
        let result = try_place_word("HELLO", 0, 3, WordDirection::Horizontal, &cells, 5);
        assert!(result.is_none());
    }

    #[test]
    fn reverse_direction_out_of_bounds_is_rejected() {
        let cells = vec![vec![EMPTY_CELL; 5]; 5];
        let result =
            try_place_word("HELLO", 2, 1, WordDirection::ReverseHorizontal, &cells, 5);
        assert!(result.is_none());
    }

    #[test]
    fn single_char_word_placement() {
        let cells = vec![vec![EMPTY_CELL; 5]; 5];
        let result = try_place_word("A", 2, 2, WordDirection::Horizontal, &cells, 5);
        assert!(result.is_some());
        assert_eq!(result.unwrap().word, "A");
    }

    // ── Selection validation ────────────────────────────────────────────

    #[test]
    fn validate_selection_returns_word_for_correct_line() {
        let mut grid = grid_from_rows(&[
            "CATXXXXX",
            "OXXXXXXX",
            "DEFGXXXX",
            "XXXXXXXX",
            "XXXXXXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "CAT".to_string(),
            start: (0, 0),
            end: (0, 2),
            direction: WordDirection::Horizontal,
        });

        let cells = vec![(0, 0), (0, 1), (0, 2)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("CAT".to_string())
        );
    }

    #[test]
    fn validate_selection_forward_word() {
        let mut grid = grid_from_rows(&[
            "CXXXXXXX",
            "OXXXXXXX",
            "DXXXXXXX",
            "EXXXXXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "CODE".to_string(),
            start: (0, 0),
            end: (3, 0),
            direction: WordDirection::Vertical,
        });

        let cells = vec![(0, 0), (1, 0), (2, 0), (3, 0)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("CODE".to_string())
        );
    }

    #[test]
    fn validate_selection_reverse_word() {
        let mut grid = grid_from_rows(&[
            "CXXXXXXX",
            "OXXXXXXX",
            "DXXXXXXX",
            "EXXXXXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "CODE".to_string(),
            start: (0, 0),
            end: (3, 0),
            direction: WordDirection::Vertical,
        });

        let cells = vec![(3, 0), (2, 0), (1, 0), (0, 0)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("CODE".to_string())
        );
    }

    #[test]
    fn validate_selection_diagonal_down_right() {
        let mut grid = grid_from_rows(&[
            "AXXXXXXX",
            "XBXXXXXX",
            "XXCXXXXX",
            "XXXXXXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "ABC".to_string(),
            start: (0, 0),
            end: (2, 2),
            direction: WordDirection::DiagonalDownRight,
        });

        let cells = vec![(0, 0), (1, 1), (2, 2)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("ABC".to_string())
        );
    }

    #[test]
    fn validate_selection_diagonal_down_left() {
        let mut grid = grid_from_rows(&[
            "XXXXXXAX",
            "XXXXXBXX",
            "XXXXCXXX",
            "XXXXXXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "ABC".to_string(),
            start: (0, 6),
            end: (2, 4),
            direction: WordDirection::DiagonalDownLeft,
        });

        let cells = vec![(0, 6), (1, 5), (2, 4)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("ABC".to_string())
        );
    }

    #[test]
    fn validate_selection_reverse_horizontal() {
        let mut grid = grid_from_rows(&["XTACXXXX"]);
        grid.placed_words.push(PlacedWord {
            word: "CAT".to_string(),
            start: (0, 3),
            end: (0, 1),
            direction: WordDirection::ReverseHorizontal,
        });

        let cells = vec![(0, 1), (0, 2), (0, 3)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("CAT".to_string())
        );
    }

    #[test]
    fn validate_selection_reverse_vertical() {
        let mut grid = grid_from_rows(&[
            "XXXXXXXX",
            "XXTXXXXX",
            "XXAXXXXX",
            "XXCXXXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "CAT".to_string(),
            start: (3, 2),
            end: (1, 2),
            direction: WordDirection::ReverseVertical,
        });

        let cells = vec![(1, 2), (2, 2), (3, 2)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("CAT".to_string())
        );
    }

    #[test]
    fn validate_selection_reverse_diagonal_down_right() {
        let mut grid = grid_from_rows(&[
            "XXXXXXXX",
            "XXXXXCXX",
            "XXXXBXXX",
            "XXXAXXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "ABC".to_string(),
            start: (3, 3),
            end: (1, 5),
            direction: WordDirection::ReverseDiagonalDownRight,
        });

        let cells = vec![(1, 5), (2, 4), (3, 3)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("ABC".to_string())
        );
    }

    #[test]
    fn validate_selection_reverse_diagonal_down_left() {
        let mut grid = grid_from_rows(&[
            "XXXXXXXX",
            "XXCXXXXX",
            "XXXBXXXX",
            "XXXXAXXX",
        ]);
        grid.placed_words.push(PlacedWord {
            word: "ABC".to_string(),
            start: (3, 4),
            end: (1, 2),
            direction: WordDirection::ReverseDiagonalDownLeft,
        });

        let cells = vec![(1, 2), (2, 3), (3, 4)];
        assert_eq!(
            validate_selection(&grid, cells),
            Some("ABC".to_string())
        );
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn single_cell_selection_returns_none() {
        let grid = grid_from_rows(&["ABC", "DEF", "GHI"]);
        assert_eq!(validate_selection(&grid, vec![(0, 0)]), None);
    }

    #[test]
    fn empty_selection_returns_none() {
        let grid = grid_from_rows(&["ABC", "DEF", "GHI"]);
        assert_eq!(validate_selection(&grid, vec![]), None);
    }

    #[test]
    fn non_linear_selection_returns_none() {
        let mut grid = grid_from_rows(&["ABC", "DEF", "GHI"]);
        grid.placed_words.push(PlacedWord {
            word: "ABC".to_string(),
            start: (0, 0),
            end: (0, 2),
            direction: WordDirection::Horizontal,
        });
        let cells = vec![(0, 0), (0, 1), (1, 1)];
        assert_eq!(validate_selection(&grid, cells), None);
    }

    #[test]
    fn selection_with_gaps_returns_none() {
        let grid = grid_from_rows(&["ABCDEF"]);
        let cells = vec![(0, 0), (0, 2)];
        assert_eq!(validate_selection(&grid, cells), None);
    }

    #[test]
    fn duplicate_cell_selection_returns_none() {
        let grid = grid_from_rows(&["ABCDEF"]);
        let cells = vec![(0, 0), (0, 0), (0, 1)];
        assert_eq!(validate_selection(&grid, cells), None);
    }

    #[test]
    fn selection_not_matching_any_word_returns_none() {
        let mut grid = grid_from_rows(&["CATDOGX"]);
        grid.placed_words.push(PlacedWord {
            word: "CAT".to_string(),
            start: (0, 0),
            end: (0, 2),
            direction: WordDirection::Horizontal,
        });
        let cells = vec![(0, 3), (0, 4), (0, 5)];
        assert_eq!(validate_selection(&grid, cells), None);
    }

    // ── Helpers for assertions ──────────────────────────────────────────

    fn assert_no_empty_cells(grid: &Grid) {
        for row in &grid.cells {
            for &cell in row {
                assert_ne!(
                    cell, EMPTY_CELL,
                    "Grid should have no empty cells after generation"
                );
            }
        }
    }

    fn assert_all_words_placed(grid: &Grid, words: &[String]) {
        let placed: Vec<String> = grid
            .placed_words
            .iter()
            .map(|pw| pw.word.clone())
            .collect();
        for word in words {
            assert!(
                placed.iter().any(|p| p.eq_ignore_ascii_case(word)),
                "Word '{}' should be placed in the grid",
                word
            );
        }
    }
}
