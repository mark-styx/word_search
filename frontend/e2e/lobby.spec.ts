import { test, expect } from "@playwright/test";
import { registerAndAuth, uniqueSuffix, createGameApi } from "./helpers";

test.describe("Lobby page", () => {
  test("redirects to login when unauthenticated", async ({ page }) => {
    await page.goto("/lobby");
    await expect(page).toHaveURL(/\/login/, { timeout: 10_000 });
  });

  test("shows lobby with create and solo buttons when authenticated", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_lobby_${suffix}`, "testpass123");

    await page.goto("/lobby");

    await expect(page.getByRole("heading", { name: "Game Lobby" })).toBeVisible();
    await expect(page.getByRole("button", { name: /Create Game/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /Solo Game/ })).toBeVisible();
  });

  test("shows empty state or game list", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_empty_${suffix}`, "testpass123");

    await page.goto("/lobby");

    // Either empty state or game cards shown (shared DB)
    const main = page.locator("main");
    await expect(
      main.getByText("No active games right now").or(main.getByText(/Host:/))
    ).toBeVisible({ timeout: 10_000 });
  });

  test("lists existing games", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_list_${suffix}`;
    const token = await registerAndAuth(page, username, "testpass123");

    const gameId = await createGameApi(token);
    await page.goto("/lobby");

    // Should see the game card with host info
    await expect(page.getByText(`Host: ${username}`)).toBeVisible({ timeout: 10_000 });
  });

  test("create game modal opens and can create a game", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_create_${suffix}`, "testpass123");

    await page.goto("/lobby");

    // Open create modal
    await page.getByRole("button", { name: /Create Game/ }).click();
    await expect(page.getByRole("heading", { name: "Create Game" })).toBeVisible();

    // Should have theme and difficulty selects
    await expect(page.getByLabel("Word Theme")).toBeVisible();
    await expect(page.getByLabel("Difficulty")).toBeVisible();

    // Click create (exact match to avoid "Create Game" button match)
    await page.getByRole("button", { name: "Create", exact: true }).click();

    // Should navigate to game page
    await expect(page).toHaveURL(/\/game\//, { timeout: 15_000 });
  });

  test("solo game button navigates to game", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_solo_${suffix}`, "testpass123");

    await page.goto("/lobby");
    await page.getByRole("button", { name: /Solo Game/ }).click();

    await expect(page).toHaveURL(/\/game\//, { timeout: 15_000 });
  });

  test("clicking a game in the list joins and navigates to it", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_join_${suffix}`, "testpass123");

    const gameId = await createGameApi(token);
    await page.goto("/lobby");

    // Click the game card with this user as host
    const hostText = page.getByText(`Host: e2e_join_${suffix}`);
    await expect(hostText).toBeVisible({ timeout: 10_000 });
    await hostText.click();

    await expect(page).toHaveURL(/\/game\//, { timeout: 15_000 });
  });

  test("share button opens QR code modal", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_qr_${suffix}`, "testpass123");

    await page.goto("/lobby");
    await page.getByRole("button", { name: /Share/ }).click();

    await expect(page.getByText("Share Server")).toBeVisible();
    await expect(page.getByText("Scan to open this server")).toBeVisible();
  });
});
