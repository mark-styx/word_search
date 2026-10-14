"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { Search, Trophy, User, LogOut, Palette, Home } from "lucide-react";
import { useState, useRef, useEffect } from "react";
import { useTheme } from "@/lib/themes-provider";
import { themes } from "@/lib/themes";
import { isAuthenticated, logout } from "@/lib/auth";

const navLinks = [
  { href: "/", label: "Home", icon: Home },
  { href: "/lobby", label: "Lobby", icon: Search },
  { href: "/leaderboard", label: "Leaderboard", icon: Trophy },
];

export default function Header() {
  const pathname = usePathname();
  const router = useRouter();
  const { theme, setTheme } = useTheme();
  const [themeOpen, setThemeOpen] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [authenticated, setAuthenticated] = useState(false);
  const themeRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close dropdowns on outside click
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (themeRef.current && !themeRef.current.contains(e.target as Node)) setThemeOpen(false);
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setMenuOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  useEffect(() => {
    setAuthenticated(isAuthenticated());
  }, [pathname]);

  const hideHeader = pathname === "/login" || pathname === "/register";

  if (hideHeader) return null;

  return (
    <header className="sticky top-0 z-40 border-b border-border bg-bg/80 pt-[env(safe-area-inset-top)] backdrop-blur-md">
      <div className="mx-auto flex min-h-14 max-w-5xl items-center justify-between px-4">
        {/* Logo */}
        <Link href="/" className="flex items-center gap-2 font-bold text-lg text-text">
          <Search size={22} className="text-primary" />
          <span>WordSearch</span>
        </Link>

        {/* Desktop nav */}
        <nav className="hidden md:flex items-center gap-1">
          {navLinks.map(({ href, label, icon: Icon }) => (
            <Link
              key={href}
              href={href}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors ${
                pathname === href
                  ? "bg-primary/10 text-primary"
                  : "text-text-secondary hover:text-text hover:bg-bg-secondary"
              }`}
            >
              <Icon size={16} />
              {label}
            </Link>
          ))}
        </nav>

        {/* Right side */}
        <div className="flex items-center gap-2">
          {/* Theme switcher */}
          <div className="relative" ref={themeRef}>
            <button
              onClick={() => setThemeOpen(!themeOpen)}
              className="rounded-lg p-2 text-text-secondary hover:bg-bg-secondary transition-colors"
              title="Change theme"
            >
              <Palette size={18} />
            </button>
            {themeOpen && (
              <div className="absolute right-0 top-full z-50 mt-1 max-h-80 w-44 overflow-y-auto rounded-lg border border-border bg-bg py-1 shadow-lg">
                {themes.map((t) => (
                  <button
                    key={t.id}
                    onClick={() => { setTheme(t.id); setThemeOpen(false); }}
                    className={`flex w-full items-center gap-2 px-3 py-2 text-sm transition-colors ${
                      t.id === theme.id ? "bg-primary/10 text-primary font-medium" : "text-text hover:bg-bg-secondary"
                    }`}
                  >
                    <span
                      className="h-4 w-4 rounded-full border border-border"
                      style={{ background: t.colors.primary }}
                    />
                    {t.name}
                  </button>
                ))}
              </div>
            )}
          </div>

          {authenticated ? (
            <div className="relative" ref={menuRef}>
              <button
                onClick={() => setMenuOpen(!menuOpen)}
                className="flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-sm text-text-secondary hover:bg-bg-secondary transition-colors"
              >
                <User size={18} />
                <span className="hidden sm:inline">Profile</span>
              </button>
              {menuOpen && (
                <div className="absolute right-0 top-full mt-1 w-44 rounded-lg border border-border bg-bg shadow-lg py-1 z-50">
                  <Link
                    href="/profile"
                    onClick={() => setMenuOpen(false)}
                    className="flex w-full items-center gap-2 px-3 py-2 text-sm text-text hover:bg-bg-secondary"
                  >
                    <User size={14} /> Profile & Stats
                  </Link>
                  <button
                    onClick={() => { logout(); setMenuOpen(false); }}
                    className="flex w-full items-center gap-2 px-3 py-2 text-sm text-error hover:bg-bg-secondary"
                  >
                    <LogOut size={14} /> Switch Player
                  </button>
                </div>
              )}
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <Link
                href="/login"
                className="rounded-lg px-3 py-1.5 text-sm font-medium text-text-secondary hover:text-text transition-colors"
              >
                Log In
              </Link>
              <Link
                href="/register"
                className="rounded-lg bg-primary px-3 py-1.5 text-sm font-medium text-white hover:bg-primary-hover transition-colors"
              >
                Sign Up
              </Link>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
