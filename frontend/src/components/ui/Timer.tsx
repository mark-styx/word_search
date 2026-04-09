"use client";

import { useEffect, useState } from "react";

interface TimerProps {
  /** Seconds remaining. Pass null for no active timer. */
  seconds: number | null;
  /** Called when timer hits 0. */
  onExpire?: () => void;
  className?: string;
}

function formatTime(total: number): string {
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export default function Timer({ seconds, onExpire, className = "" }: TimerProps) {
  const [remaining, setRemaining] = useState(seconds);

  useEffect(() => {
    setRemaining(seconds);
  }, [seconds]);

  useEffect(() => {
    if (remaining === null || remaining <= 0) return;
    const id = setInterval(() => {
      setRemaining((prev) => {
        if (prev === null || prev <= 1) {
          clearInterval(id);
          onExpire?.();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(id);
  }, [remaining, onExpire]);

  if (remaining === null) return null;

  const pct = seconds ? (remaining / seconds) * 100 : 0;
  const urgent = remaining <= 30;

  return (
    <div className={`flex flex-col items-center gap-1 ${className}`}>
      <span
        className={`text-2xl font-mono font-bold tabular-nums ${
          urgent ? "text-error animate-pulse" : "text-text"
        }`}
      >
        {formatTime(remaining)}
      </span>
      {seconds && seconds > 0 && (
        <div className="w-full h-1.5 rounded-full bg-bg-secondary overflow-hidden">
          <div
            className={`h-full rounded-full transition-all duration-1000 ease-linear ${
              urgent ? "bg-error" : "bg-primary"
            }`}
            style={{ width: `${pct}%` }}
          />
        </div>
      )}
    </div>
  );
}
