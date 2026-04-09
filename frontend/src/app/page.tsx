"use client";

import Link from "next/link";
import { Search, Trophy, Users, Zap, BookOpen } from "lucide-react";
import { PageContainer } from "@/components/layout";
import { Button, Card } from "@/components/ui";
import { isAuthenticated } from "@/lib/auth";

const features = [
  { icon: Users, title: "Multiplayer", desc: "Play with friends on your local network" },
  { icon: Zap, title: "Real-Time", desc: "See words get found the instant they happen" },
  { icon: BookOpen, title: "10 Themes", desc: "Animals, Science, Movies, and more" },
  { icon: Trophy, title: "Leaderboard", desc: "Track your scores and compete for the top" },
];

export default function Home() {
  const authed = isAuthenticated();

  return (
    <PageContainer className="flex flex-col items-center justify-center gap-10 py-16">
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
            <Link href="/register">
              <Button size="lg">Get Started</Button>
            </Link>
            <Link href="/login">
              <Button variant="secondary" size="lg">Log In</Button>
            </Link>
          </>
        )}
      </div>

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
    </PageContainer>
  );
}
