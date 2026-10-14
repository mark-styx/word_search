"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Search, Trophy, Users, Zap, BookOpen } from "lucide-react";
import { PageContainer } from "@/components/layout";
import ServerDiscoveryCard from "@/components/discovery/ServerDiscoveryCard";
import LeaderboardCharts from "@/components/stats/LeaderboardCharts";
import InstallAppPrompt from "@/components/system/InstallAppPrompt";
import { Button, Card } from "@/components/ui";
import { isAuthenticated } from "@/lib/auth";
import { api } from "@/lib/api";
import type { LeaderboardEntry } from "@/types";

const features = [
  { icon: Users, title: "Multiplayer", desc: "Play with friends on your local network" },
  { icon: Zap, title: "Real-Time", desc: "See words get found the instant they happen" },
  { icon: BookOpen, title: "31 Themes", desc: "Boating, camping, cats, fishing, pizza, recipe ingredients, luxury brands, snacks, fast food, home renovation, mountains, ocean, and more" },
  { icon: Trophy, title: "Leaderboard", desc: "Track your scores and compete for the top" },
];

export default function Home() {
  const [authed, setAuthed] = useState(false);
  const [leaderboardEntries, setLeaderboardEntries] = useState<LeaderboardEntry[]>([]);

  useEffect(() => {
    setAuthed(isAuthenticated());
    void api
      .getLeaderboard(1, 10)
      .then((result) => {
        setLeaderboardEntries((result as { entries: LeaderboardEntry[] }).entries ?? []);
      })
      .catch(() => {});
  }, []);

  return (
    <PageContainer className="flex flex-col items-center justify-center gap-10 py-16">
      <InstallAppPrompt />
      <div className="flex flex-col items-center gap-4 text-center">
        <div className="flex items-center gap-3">
          <Search size={48} className="text-primary" />
          <h1 className="text-4xl font-bold text-text sm:text-5xl">WordSearch</h1>
        </div>
        <p className="max-w-md text-lg text-text-secondary">
          A real-time multiplayer word search game. Find words, beat the clock,
          compete with friends on your LAN.
        </p>
      </div>

      <div className="flex flex-wrap justify-center gap-3">
        {authed ? (
          <>
            <Link href="/lobby">
              <Button size="lg">Join Lobby</Button>
            </Link>
            <Link href="/solo">
              <Button variant="secondary" size="lg">Solo Game</Button>
            </Link>
          </>
        ) : (
          <>
            <Link href="/login">
              <Button size="lg">Play Now</Button>
            </Link>
          </>
        )}
      </div>

      {!authed && (
        <p className="max-w-md text-center text-sm text-text-secondary">
          Pick an existing player, create a new one, or jump in as a guest from the next screen.
        </p>
      )}

      <ServerDiscoveryCard />

      <div className="grid w-full max-w-2xl grid-cols-1 gap-4 sm:grid-cols-2">
        {features.map(({ icon: Icon, title, desc }) => (
          <Card key={title} className="flex items-start gap-3">
            <div className="rounded-lg bg-primary/10 p-2">
              <Icon size={20} className="text-primary" />
            </div>
            <div>
              <h3 className="font-semibold text-text">{title}</h3>
              <p className="text-sm text-text-secondary">{desc}</p>
            </div>
          </Card>
        ))}
      </div>

      <div className="w-full max-w-5xl">
        <LeaderboardCharts
          entries={leaderboardEntries}
          metrics={["total_score", "games_won"]}
          title="Leaderboard Snapshot"
          description="Live leaders from this server, split by points and wins."
          compact
        />
        <div className="mt-4 flex justify-center">
          <Link href="/leaderboard">
            <Button variant="secondary">Open Full Leaderboard</Button>
          </Link>
        </div>
      </div>
    </PageContainer>
  );
}
