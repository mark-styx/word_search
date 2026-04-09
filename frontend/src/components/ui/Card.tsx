"use client";

import { ReactNode } from "react";

interface CardProps {
  children: ReactNode;
  className?: string;
  onClick?: () => void;
  hoverable?: boolean;
}

export default function Card({ children, className = "", onClick, hoverable }: CardProps) {
  return (
    <div
      className={`rounded-xl border border-border bg-bg p-4 shadow-sm ${
        hoverable ? "cursor-pointer transition-shadow hover:shadow-md hover:border-primary/30" : ""
      } ${className}`}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={onClick ? (e) => { if (e.key === "Enter" || e.key === " ") onClick(); } : undefined}
    >
      {children}
    </div>
  );
}
