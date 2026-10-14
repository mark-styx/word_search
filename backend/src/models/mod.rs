// Shared data structures
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordDirection {
    Horizontal,
    Vertical,
    DiagonalDownRight,
    DiagonalDownLeft,
    ReverseHorizontal,
    ReverseVertical,
    ReverseDiagonalDownRight,
    ReverseDiagonalDownLeft,
}

impl WordDirection {
    pub fn delta(&self) -> (isize, isize) {
        match self {
            Self::Horizontal => (0, 1),
            Self::Vertical => (1, 0),
            Self::DiagonalDownRight => (1, 1),
            Self::DiagonalDownLeft => (1, -1),
            Self::ReverseHorizontal => (0, -1),
            Self::ReverseVertical => (-1, 0),
            Self::ReverseDiagonalDownRight => (-1, -1),
            Self::ReverseDiagonalDownLeft => (-1, 1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacedWord {
    pub word: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub direction: WordDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    pub cells: Vec<Vec<char>>,
    pub width: usize,
    pub height: usize,
    pub placed_words: Vec<PlacedWord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl Difficulty {
    pub fn grid_size(&self) -> usize {
        match self {
            Self::Easy => 8,
            Self::Medium => 12,
            Self::Hard => 15,
            Self::Expert => 20,
        }
    }

    pub fn word_count(&self) -> usize {
        match self {
            Self::Easy => 8,
            Self::Medium => 13,
            Self::Hard => 18,
            Self::Expert => 24,
        }
    }

    pub fn score_multiplier(&self) -> u32 {
        match self {
            Self::Easy => 1,
            Self::Medium => 2,
            Self::Hard => 3,
            Self::Expert => 5,
        }
    }

    pub fn allowed_directions(&self) -> Vec<WordDirection> {
        use WordDirection::*;
        match self {
            Self::Easy => vec![Horizontal, Vertical],
            Self::Medium => vec![Horizontal, Vertical, DiagonalDownRight, DiagonalDownLeft],
            Self::Hard => vec![
                Horizontal,
                Vertical,
                DiagonalDownRight,
                DiagonalDownLeft,
                ReverseHorizontal,
                ReverseVertical,
            ],
            Self::Expert => vec![
                Horizontal,
                Vertical,
                DiagonalDownRight,
                DiagonalDownLeft,
                ReverseHorizontal,
                ReverseVertical,
                ReverseDiagonalDownRight,
                ReverseDiagonalDownLeft,
            ],
        }
    }

    pub fn score_for_word(&self, word: &str) -> u32 {
        let length = word.chars().count() as u32;
        let short_word_bonus = ((12u32.saturating_sub(length) + 1) / 2) + 2;
        short_word_bonus * self.score_multiplier()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordTheme {
    Animals,
    Cats,
    Boating,
    Camping,
    RecipeIngredients,
    Sports,
    Food,
    Fishing,
    Pizza,
    Snacks,
    FastFood,
    Bugs,
    Movies,
    Cars,
    Desert,
    Mountains,
    Ocean,
    Weather,
    Garden,
    Farm,
    ShinyThings,
    Toys,
    Space,
    Science,
    Geography,
    Technology,
    Music,
    History,
    HomeRenovation,
    LuxuryBrands,
    Nature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WordBankSource {
    BuiltIn { theme: WordTheme },
    Custom { id: Uuid },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Lobby,
    InProgress,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameFinishReason {
    Completed,
    Timeout,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: Uuid,
    pub username: String,
    pub score: u32,
    pub is_host: bool,
    #[serde(default)]
    pub ready: bool,
    pub highlight_color: String,
    #[serde(default = "player_connected_default")]
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub id: Uuid,
    pub grid: Option<Grid>,
    pub theme: String,
    pub word_bank_source: WordBankSource,
    pub difficulty: Difficulty,
    pub players: HashMap<Uuid, PlayerState>,
    pub found_words: HashMap<Uuid, Vec<String>>,
    pub status: GameStatus,
    #[serde(default)]
    pub finished_reason: Option<GameFinishReason>,
    pub started_at: Option<DateTime<Utc>>,
    pub time_limit: Option<Duration>,
    pub max_players: usize,
    pub revision: u64,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    pub word_claims: HashMap<String, WordClaim>,
}

#[derive(Debug, Clone)]
pub struct WordClaim {
    pub player_id: Uuid,
    pub claimed_at: DateTime<Utc>,
}

impl GameState {
    pub fn touch(&mut self) {
        self.revision = self.revision.saturating_add(1);
        self.updated_at = Utc::now();
    }
}

fn player_connected_default() -> bool {
    true
}

// ── WebSocket message protocol ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    SelectCells { cells: Vec<(usize, usize)> },
    Chat { message: String },
    Ready,
    LeaveGame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    PlayerJoined {
        player: PlayerState,
    },
    PlayerLeft {
        player_id: Uuid,
    },
    PlayerReady {
        player_id: Uuid,
    },
    GameStarting {
        grid: Grid,
        words_to_find: Vec<String>,
        time_limit: Option<u64>,
    },
    WordFound {
        player_id: Uuid,
        word: String,
        score: u32,
    },
    GameFinished {
        rankings: Vec<PlayerState>,
        reason: GameFinishReason,
    },
    GameStateSync {
        game: serde_json::Value,
    },
    PlayerScoreUpdate {
        player_id: Uuid,
        score: u32,
        found_count: usize,
    },
    ChatMessage {
        player_id: Uuid,
        message: String,
        timestamp: DateTime<Utc>,
    },
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::Difficulty;

    #[test]
    fn shorter_words_score_more_than_longer_words() {
        let difficulty = Difficulty::Medium;

        assert!(difficulty.score_for_word("CAR") > difficulty.score_for_word("ENGINE"));
        assert!(difficulty.score_for_word("STAR") > difficulty.score_for_word("SATELLITE"));
    }

    #[test]
    fn score_for_word_has_a_minimum_floor() {
        let difficulty = Difficulty::Expert;
        assert_eq!(difficulty.score_for_word("CONSTELLATION"), 10);
    }
}
