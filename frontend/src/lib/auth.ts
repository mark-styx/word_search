import { getAccessToken, clearTokens, api } from "./api";
import type { User } from "@/types";

export { getAccessToken, setAccessToken, clearTokens, api } from "./api";

export function isAuthenticated(): boolean {
  return !!getAccessToken();
}

export function logout() {
  clearTokens();
  if (typeof window !== "undefined") {
    window.location.href = "/login";
  }
}
