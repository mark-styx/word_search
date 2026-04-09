// Shared data structures
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
            Self::Easy => 6,
            Self::Medium => 10,
            Self::Hard => 15,
            Self::Expert => 20,
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
                Horizontal, Vertical, DiagonalDownRight, DiagonalDownLeft,
                ReverseHorizontal, ReverseVertical,
            ],
            Self::Expert => vec![
                Horizontal, Vertical, DiagonalDownRight, DiagonalDownLeft,
                ReverseHorizontal, ReverseVertical,
                ReverseDiagonalDownRight, ReverseDiagonalDownLeft,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordTheme {
    Animals,
    Sports,
    Food,
    Movies,
    Science,
    Geography,
    Technology,
    Music,
    History,
    Nature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Lobby,
    InProgress,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub id: Uuid,
    pub username: String,
    pub score: u32,
    pub is_host: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub id: Uuid,
    pub grid: Option<Grid>,
    pub theme: WordTheme,
    pub difficulty: Difficulty,
    pub players: HashMap<Uuid, PlayerState>,
    pub found_words: HashMap<Uuid, Vec<String>>,
    pub status: GameStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub time_limit: Option<Duration>,
    pub max_players: usize,
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
    PlayerJoined { player: PlayerState },
    PlayerLeft { player_id: Uuid },
    PlayerReady { player_id: Uuid },
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
    Error { message: String },
}
