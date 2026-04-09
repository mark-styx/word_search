import { test, expect } from "@playwright/test";

test.describe("Home page", () => {
  test("shows app title and features when unauthenticated", async ({ page }) => {
    await page.goto("/");

    await expect(page.getByRole("heading", { name: "WordSearch" })).toBeVisible();
    await expect(page.getByText("real-time multiplayer word search")).toBeVisible();
    await expect(page.getByRole("link", { name: "Get Started" })).toBeVisible();
    // "Log In" appears in header and hero - use .first()
    await expect(page.getByRole("link", { name: "Log In" }).first()).toBeVisible();

    // Feature headings (use exact: true to avoid matching body text)
    const main = page.locator("main");
    await expect(main.getByRole("heading", { name: "Multiplayer", exact: true })).toBeVisible();
    await expect(main.getByRole("heading", { name: "Real-Time", exact: true })).toBeVisible();
    await expect(main.getByRole("heading", { name: "10 Themes", exact: true })).toBeVisible();
    await expect(main.getByRole("heading", { name: "Leaderboard", exact: true })).toBeVisible();
  });

  test("header shows Login and Sign Up when unauthenticated", async ({ page }) => {
    await page.goto("/");

    const header = page.locator("header");
    await expect(header.getByRole("link", { name: "Log In" })).toBeVisible();
    await expect(header.getByRole("link", { name: "Sign Up" })).toBeVisible();
  });

  test("unauthenticated user can navigate to register", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("link", { name: "Get Started" }).click();
    await expect(page).toHaveURL(/\/register/);
  });

  test("unauthenticated user can navigate to login", async ({ page }) => {
    await page.goto("/");
    // Click the hero Log In link (inside main)
    await page.locator("main").getByRole("link", { name: "Log In" }).click();
    await expect(page).toHaveURL(/\/login/);
  });
});
