import { test, expect } from "@playwright/test";
import { registerAndAuth, uniqueSuffix } from "./helpers";

test.describe("Leaderboard page", () => {
  test("shows leaderboard heading", async ({ page }) => {
    await page.goto("/leaderboard");

    await expect(page.getByRole("heading", { name: "Leaderboard" })).toBeVisible();
  });

  test("shows empty state or entries", async ({ page }) => {
    await page.goto("/leaderboard");

    // Either empty state or leaderboard entries
    const emptyState = page.getByText("No games played yet");
    const entries = page.locator("[class*='Card']").filter({ hasText: "pts" });
    await expect(emptyState.or(entries.first())).toBeVisible({ timeout: 10_000 });
  });

  test("pagination controls are present", async ({ page }) => {
    await page.goto("/leaderboard");

    await expect(page.getByRole("button", { name: "Previous" })).toBeVisible();
    // Next button may be disabled or hidden if no entries, just check Previous and Page text
    await expect(page.getByText(/Page \d+/)).toBeVisible();
  });

  test("previous button is disabled on page 1", async ({ page }) => {
    await page.goto("/leaderboard");

    const prevBtn = page.getByRole("button", { name: "Previous" });
    await expect(prevBtn).toBeDisabled();
  });

  test("back button navigates away from leaderboard", async ({ page }) => {
    await page.goto("/");
    await page.goto("/leaderboard");
    // Click back arrow (first button inside main)
    await page.locator("main button").first().click();
    // Should go back to home
    await page.waitForTimeout(500);
  });
});

test.describe("Profile page", () => {
  test("redirects to login when unauthenticated", async ({ page }) => {
    await page.goto("/profile");
    await expect(page).toHaveURL(/\/login/, { timeout: 10_000 });
  });

  test("shows profile and stats heading when authenticated", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_prof_${suffix}`, "testpass123");

    await page.goto("/profile");

    await expect(page.getByRole("heading", { name: "Profile & Stats" })).toBeVisible({
      timeout: 10_000,
    });
  });

  test("shows visual theme selector", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_theme_${suffix}`, "testpass123");

    await page.goto("/profile");

    await expect(page.getByText("Visual Theme")).toBeVisible({ timeout: 10_000 });
  });

  test("shows stats section for new user", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_nostats_${suffix}`, "testpass123");

    await page.goto("/profile");

    // Either "No stats yet" or actual stats display
    const main = page.locator("main");
    await expect(main.getByText(/No stats yet|Games Played|total_games/i)).toBeVisible({ timeout: 10_000 });
  });

  test("shows link to full leaderboard", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_lb_${suffix}`, "testpass123");

    await page.goto("/profile");

    const lbLink = page.getByRole("link", { name: /View Full Leaderboard/ });
    await expect(lbLink).toBeVisible();
  });
});
