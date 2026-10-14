export interface User {
  id: string;
  username: string;
  created_at?: string;
  visual_theme?: string;
  preferred_word_theme?: string | null;
  highlight_color?: string;
  has_recovery_pin?: boolean;
  is_guest?: boolean;
}

export interface PlayerDirectoryEntry {
  id: string;
  username: string;
  created_at: string;
}

export interface PlayerState {
  id: string;
  username: string;
  score: number;
  is_host: boolean;
  ready: boolean;
  highlight_color: string;
  connected: boolean;
}

export type WordDirection =
  | "horizontal"
  | "vertical"
  | "diagonal_down_right"
  | "diagonal_down_left"
  | "reverse_horizontal"
  | "reverse_vertical"
  | "reverse_diagonal_down_right"
  | "reverse_diagonal_down_left";

export interface PlacedWord {
  word: string;
  start: [number, number];
  end: [number, number];
  direction: WordDirection;
}

export interface Grid {
  cells: string[][];
  width: number;
  height: number;
  placed_words: PlacedWord[];
}

export type Difficulty = "easy" | "medium" | "hard" | "expert";

export type WordTheme =
  | "animals"
  | "cats"
  | "boating"
  | "camping"
  | "recipe_ingredients"
  | "sports"
  | "food"
  | "fishing"
  | "pizza"
  | "snacks"
  | "fast_food"
  | "bugs"
  | "movies"
  | "cars"
  | "desert"
  | "mountains"
  | "ocean"
  | "weather"
  | "garden"
  | "farm"
  | "shiny_things"
  | "toys"
  | "space"
  | "science"
  | "geography"
  | "technology"
  | "music"
  | "history"
  | "home_renovation"
  | "luxury_brands"
  | "nature";

export type GameStatus = "lobby" | "in_progress" | "finished";
export type GameFinishReason = "completed" | "timeout" | "abandoned";

export interface GameState {
  id: string;
  grid: Grid | null;
  theme: string;
  difficulty: Difficulty;
  players: PlayerState[];
  found_words: Record<string, string[]>;
  status: GameStatus;
  finished_reason: GameFinishReason | null;
  started_at: string | null;
  time_limit_secs: number | null;
  max_players: number;
  revision: number;
  updated_at: string;
}

export interface GameLobby {
  id: string;
  theme: string;
  difficulty: Difficulty;
  host: string;
  player_count: number;
  max_players: number;
  revision: number;
  updated_at: string;
}

export interface UserStats {
  total_games: number;
  games_won: number;
  total_score: number;
  best_single_game_score: number;
  total_words_found: number;
  contested_words_won: number;
  contested_words_lost: number;
  closest_contest_ms: number | null;
  fastest_word_find_ms: number | null;
  average_word_find_ms: number | null;
}

export interface LeaderboardEntry {
  username: string;
  total_score: number;
  games_won: number;
  total_games: number;
}

export interface GameHistoryEntry {
  game_id: string;
  theme: string;
  difficulty: Difficulty;
  rank: number | null;
  score: number;
  words_found: number;
  finished_at: string | null;
}

export type WordBankStatus = "ready" | "generating" | "failed";
export type WordBankSource = "builtin" | "custom";
export type WordBankExtensionState = "idle" | "extending" | "disabled" | "failed";

export interface WordBank {
  id: string;
  key: string;
  label: string;
  source: WordBankSource;
  status: WordBankStatus;
  total_words: number;
  used_words_count: number;
  total_usage_count: number;
  coverage_percent: number;
  can_extend: boolean;
  extension_state: WordBankExtensionState;
  error_message: string | null;
  last_extension_error: string | null;
  created_at: string | null;
  owner_username: string | null;
  is_owner: boolean;
  is_shared: boolean;
}

// WebSocket message types
export type ClientMessage =
  | { type: "select_cells"; cells: [number, number][] }
  | { type: "chat"; message: string }
  | { type: "ready" }
  | { type: "leave_game" };

export type ServerMessage =
  | { type: "player_joined"; player: PlayerState }
  | { type: "player_left"; player_id: string }
  | { type: "player_ready"; player_id: string }
  | {
      type: "game_starting";
      grid: Grid;
      words_to_find: string[];
      time_limit: number | null;
    }
  | { type: "word_found"; player_id: string; word: string; score: number }
  | { type: "game_finished"; rankings: PlayerState[]; reason: GameFinishReason }
  | { type: "game_state_sync"; game: GameState }
  | { type: "player_score_update"; player_id: string; score: number; found_count: number }
  | { type: "chat_message"; player_id: string; message: string; timestamp: string }
  | { type: "error"; message: string };
