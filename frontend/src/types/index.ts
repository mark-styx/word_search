export interface User {
  id: string;
  username: string;
  created_at: string;
  visual_theme: string;
  preferred_word_theme: string | null;
}

export interface PlayerState {
  id: string;
  username: string;
  score: number;
  is_host: boolean;
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
  | "sports"
  | "food"
  | "movies"
  | "science"
  | "geography"
  | "technology"
  | "music"
  | "history"
  | "nature";

export type GameStatus = "lobby" | "in_progress" | "finished";

export interface GameState {
  id: string;
  grid: Grid | null;
  theme: WordTheme;
  difficulty: Difficulty;
  players: Record<string, PlayerState>;
  found_words: Record<string, string[]>;
  status: GameStatus;
  started_at: string | null;
  time_limit: number | null;
  max_players: number;
}

export interface GameLobby {
  id: string;
  theme: WordTheme;
  difficulty: Difficulty;
  host: string;
  player_count: number;
  max_players: number;
}

export interface UserStats {
  total_games: number;
  games_won: number;
  total_score: number;
  best_single_game_score: number;
  total_words_found: number;
}

export interface LeaderboardEntry {
  username: string;
  total_score: number;
  games_won: number;
  total_games: number;
}

export interface GameHistoryEntry {
  id: string;
  theme: WordTheme;
  difficulty: Difficulty;
  rank: number;
  score: number;
  words_found: number;
  finished_at: string;
}

// WebSocket message types
export type ClientMessage =
  | { type: "select_cells"; cells: { row: number; col: number }[] }
  | { type: "chat"; text: string }
  | { type: "ready" }
  | { type: "leave_game" };

export type ServerMessage =
  | { type: "player_joined"; player_id: string; username: string }
  | { type: "player_left"; player_id: string }
  | { type: "player_ready"; player_id: string }
  | { type: "game_starting" }
  | { type: "word_found"; player_id: string; word: string; score: number }
  | { type: "game_finished"; winner_id: string }
  | { type: "player_score_update"; player_id: string; score: number }
  | { type: "chat_message"; username: string; text: string }
  | { type: "error"; message: string };
