"use client";

import type { User } from "@/types";
import { api, clearTokens, getAccessToken, setAccessToken } from "./api";

export type SavedProfile = {
  id?: string;
  username: string;
  token?: string;
  secret?: string;
  recoveryPin?: string;
  createdAt?: string;
  lastUsedAt: string;
};

const SAVED_PROFILES_KEY = "saved_profiles_v1";
const MAX_SAVED_PROFILES = 24;

function canUseStorage() {
  return typeof window !== "undefined";
}

function normalizeUsername(value: string) {
  return value.trim().toLowerCase();
}

function sortProfiles(profiles: SavedProfile[]) {
  return [...profiles].sort((a, b) => {
    const aTime = Date.parse(a.lastUsedAt || "") || 0;
    const bTime = Date.parse(b.lastUsedAt || "") || 0;
    return bTime - aTime;
  });
}

export function loadSavedProfiles(): SavedProfile[] {
  if (!canUseStorage()) return [];

  try {
    const raw = window.localStorage.getItem(SAVED_PROFILES_KEY);
    if (!raw) return [];

    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];

    return sortProfiles(
      parsed.filter((profile): profile is SavedProfile => {
        return !!profile && typeof profile.username === "string" && typeof profile.lastUsedAt === "string";
      })
    );
  } catch {
    return [];
  }
}

function saveSavedProfiles(profiles: SavedProfile[]) {
  if (!canUseStorage()) return;
  window.localStorage.setItem(
    SAVED_PROFILES_KEY,
    JSON.stringify(sortProfiles(profiles).slice(0, MAX_SAVED_PROFILES))
  );
}

export function rememberProfileSession(
  user: User,
  token: string,
  options: { secret?: string; recoveryPin?: string } = {}
) {
  const profiles = loadSavedProfiles();
  const now = new Date().toISOString();
  const nextProfile: SavedProfile = {
    id: user.id,
    username: user.username,
    token,
    secret: options.secret,
    recoveryPin: options.recoveryPin,
    createdAt: user.created_at,
    lastUsedAt: now,
  };

  const matchIndex = profiles.findIndex((profile) => {
    if (user.id && profile.id) {
      return profile.id === user.id;
    }
    return normalizeUsername(profile.username) === normalizeUsername(user.username);
  });

  if (matchIndex >= 0) {
    const existing = profiles[matchIndex];
    profiles[matchIndex] = {
      ...existing,
      ...nextProfile,
      secret: options.secret ?? existing.secret,
      recoveryPin: options.recoveryPin ?? existing.recoveryPin,
    };
  } else {
    profiles.push(nextProfile);
  }

  saveSavedProfiles(profiles);
}

export function rememberCurrentSession(
  user: User,
  options: { secret?: string; recoveryPin?: string } = {}
) {
  const token = getAccessToken();
  if (!token) return;
  rememberProfileSession(user, token, options);
}

export function forgetSavedProfile(profile: SavedProfile) {
  const profiles = loadSavedProfiles().filter((entry) => {
    if (profile.id && entry.id) {
      return entry.id !== profile.id;
    }
    return normalizeUsername(entry.username) !== normalizeUsername(profile.username);
  });
  saveSavedProfiles(profiles);
}

export function generateProfileSecret() {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return `wordsearch-${crypto.randomUUID().replace(/-/g, "")}`;
  }
  return `wordsearch-${Math.random().toString(36).slice(2)}${Date.now().toString(36)}`;
}

export async function restoreSavedProfile(profile: SavedProfile) {
  if (profile.secret) {
    clearTokens();
    const { token, user } = await api.login(profile.username, profile.secret);
    setAccessToken(token);
    rememberProfileSession(user, token, { secret: profile.secret, recoveryPin: profile.recoveryPin });
    return user;
  }

  if (profile.recoveryPin) {
    clearTokens();
    const { token, user } = await api.recover(profile.username, profile.recoveryPin);
    setAccessToken(token);
    rememberProfileSession(user, token, { recoveryPin: profile.recoveryPin });
    return user;
  }

  if (profile.token) {
    setAccessToken(profile.token);
    try {
      const user = await api.getMe();
      rememberProfileSession(user, profile.token, { recoveryPin: profile.recoveryPin });
      return user;
    } catch {
      clearTokens();
    }
  }

  throw new Error("That saved player can’t be restored on this device anymore.");
}
