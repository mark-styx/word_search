"use client";

import { useState, FormEvent } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { Trophy, ArrowLeft, Palette, BarChart3, History } from "lucide-react";
import { PageContainer } from "@/components/layout";
import { Button, Card, Select, Badge } from "@/components/ui";
import { useTheme } from "@/lib/themes-provider";
import { themes } from "@/lib/themes";
import { api, getAccessToken } from "@/lib/api";
import type { UserStats, GameHistoryEntry } from "@/types";

const difficultyLabels: Record<string, string> = {
  easy: "Easy",
  medium: "Medium",
  hard: "Hard",
  expert: "Expert",
};

export default function ProfilePage() {
  const router = useRouter();
  const { theme, setTheme } = useTheme();
  const [stats, setStats] = useState<UserStats | null>(null);
  const [history, setHistory] = useState<GameHistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);

  // Fetch stats on mount
  useState(() => {
    if (!getAccessToken()) {
      router.push("/login");
      return;
    }
    Promise.all([api.getMyStats(), api.getMyHistory(1, 10)])
      .then(([s, h]) => {
        setStats(s as UserStats);
        setHistory((h as { entries: GameHistoryEntry[] }).entries ?? []);
      })
      .catch(() => {
        // will redirect via 401 handler
      })
      .finally(() => setLoading(false));
  });

  if (loading) {
    return (
      <PageContainer className="flex items-center justify-center">
        <div className="text-text-secondary">Loading profile...</div>
      </PageContainer>
    );
  }

  return (
    <PageContainer className="flex flex-col gap-6 py-8">
      <div className="flex items-center gap-3">
        <button
          onClick={() => router.back()}
          className="rounded-lg p-2 text-text-secondary hover:bg-bg-secondary transition-colors"
        >
          <ArrowLeft size={20} />
        </button>
        <h1 className="text-2xl font-bold text-text">Profile & Stats</h1>
      </div>

      {/* Theme selection */}
      <Card>
        <div className="flex items-center gap-2 mb-3">
          <Palette size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Visual Theme</h2>
        </div>
        <Select
          value={theme.id}
          onChange={(e) => setTheme(e.target.value)}
          options={themes.map((t) => ({ value: t.id, label: t.name }))}
        />
        <div className="mt-3 flex gap-2">
          {themes.map((t) => (
            <button
              key={t.id}
              onClick={() => setTheme(t.id)}
              className={`h-8 w-8 rounded-full border-2 transition-all ${
                t.id === theme.id ? "border-primary scale-110" : "border-border"
              }`}
              style={{ background: t.colors.primary }}
              title={t.name}
            />
          ))}
        </div>
      </Card>

      {/* Stats */}
      <Card>
        <div className="flex items-center gap-2 mb-4">
          <BarChart3 size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Your Stats</h2>
        </div>
        {stats ? (
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
            <StatItem label="Games Played" value={stats.total_games} />
            <StatItem label="Games Won" value={stats.games_won} />
            <StatItem label="Total Score" value={stats.total_score} />
            <StatItem label="Best Score" value={stats.best_single_game_score} />
            <StatItem label="Words Found" value={stats.total_words_found} />
            <StatItem
              label="Win Rate"
              value={stats.total_games > 0 ? `${Math.round((stats.games_won / stats.total_games) * 100)}%` : "-"}
            />
          </div>
        ) : (
          <p className="text-text-secondary">No stats yet. Play some games!</p>
        )}
      </Card>

      {/* History */}
      <Card>
        <div className="flex items-center gap-2 mb-4">
          <History size={18} className="text-primary" />
          <h2 className="font-semibold text-text">Recent Games</h2>
        </div>
        {history.length > 0 ? (
          <div className="flex flex-col gap-2">
            {history.map((g) => (
              <div
                key={g.id}
                className="flex items-center justify-between rounded-lg border border-border px-3 py-2"
              >
                <div className="flex items-center gap-2">
                  <Badge variant={g.rank === 1 ? "success" : "default"}>
                    #{g.rank}
                  </Badge>
                  <span className="text-sm text-text capitalize">{g.theme}</span>
                  <Badge variant="primary">{difficultyLabels[g.difficulty] || g.difficulty}</Badge>
                </div>
                <div className="flex items-center gap-3 text-sm text-text-secondary">
                  <span>{g.words_found} words</span>
                  <span className="font-medium text-text">{g.score} pts</span>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-text-secondary">No games played yet.</p>
        )}
      </Card>

      <Link href="/leaderboard">
        <Button variant="secondary" className="w-full">
          <Trophy size={16} />
          View Full Leaderboard
        </Button>
      </Link>
    </PageContainer>
  );
}

function StatItem({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex flex-col">
      <span className="text-2xl font-bold text-text">{value}</span>
      <span className="text-xs text-text-secondary">{label}</span>
    </div>
  );
}
