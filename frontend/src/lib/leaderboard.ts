import type { LeaderboardEntry } from "@/types";

export type LeaderboardMetricKey =
  | "total_score"
  | "games_won"
  | "points_per_game"
  | "win_rate";

export const leaderboardMetricLabels: Record<LeaderboardMetricKey, string> = {
  total_score: "Points",
  games_won: "Wins",
  points_per_game: "Points / Game",
  win_rate: "Win %",
};

export function getLeaderboardMetricValue(entry: LeaderboardEntry, metric: LeaderboardMetricKey) {
  switch (metric) {
    case "total_score":
      return entry.total_score;
    case "games_won":
      return entry.games_won;
    case "points_per_game":
      return entry.total_games > 0 ? entry.total_score / entry.total_games : 0;
    case "win_rate":
      return entry.total_games > 0 ? (entry.games_won / entry.total_games) * 100 : 0;
  }
}

export function formatLeaderboardMetricValue(value: number, metric: LeaderboardMetricKey) {
  switch (metric) {
    case "total_score":
      return `${Math.round(value).toLocaleString()} pts`;
    case "games_won":
      return `${Math.round(value).toLocaleString()} wins`;
    case "points_per_game":
      return `${value.toFixed(value >= 100 ? 0 : 1)} pts`;
    case "win_rate":
      return `${Math.round(value)}%`;
  }
}

export function sortLeaderboardEntries(
  entries: LeaderboardEntry[],
  metric: LeaderboardMetricKey
) {
  return [...entries].sort((left, right) => {
    const delta = getLeaderboardMetricValue(right, metric) - getLeaderboardMetricValue(left, metric);
    if (delta !== 0) {
      return delta;
    }

    return left.username.localeCompare(right.username);
  });
}
