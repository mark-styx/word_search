import { test, expect } from "@playwright/test";
import {
  registerAndAuth,
  uniqueSuffix,
  createSoloGameApi,
  createGameApi,
} from "./helpers";

test.describe("Game page", () => {
  test("redirects to login when unauthenticated", async ({ page }) => {
    await page.goto("/game/some-fake-id");
    await expect(page).toHaveURL(/\/login/, { timeout: 10_000 });
  });

  test("shows error for nonexistent game", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_gnf_${suffix}`, "testpass123");

    await page.goto("/game/00000000-0000-0000-0000-000000000000");

    // App renders a friendly not-found state in main.
    await expect(
      page.locator("main").locator("text=/Game not found|HTTP 404|Failed to fetch/i")
    ).toBeVisible({ timeout: 10_000 });
  });

  test("solo game loads and shows grid", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_sg_${suffix}`, "testpass123");

    const gameId = await createSoloGameApi(token);
    await page.goto(`/game/${gameId}`);

    // Grid cells rendered with data-row/data-col attributes
    const cells = page.locator("[data-row][data-col]");
    await expect(cells.first()).toBeVisible({ timeout: 15_000 });

    // Word list and chat area present
    await expect(page.getByPlaceholder("Type a message...")).toBeVisible({ timeout: 10_000 });
  });

  test("lobby state shows waiting for players and start button", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_lob_${suffix}`, "testpass123");

    const gameId = await createGameApi(token);
    await page.goto(`/game/${gameId}`);

    await expect(page.getByText("Waiting for players")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole("button", { name: "Start Game" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Ready" })).toBeVisible();
  });

  test("host can start a multiplayer game", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_hs_${suffix}`, "testpass123");

    const gameId = await createGameApi(token);
    await page.goto(`/game/${gameId}`);
    await expect(page.getByText("Waiting for players")).toBeVisible({ timeout: 10_000 });

    await page.getByRole("button", { name: "Start Game" }).click();

    // Grid cells appear once game starts
    const cells = page.locator("[data-row][data-col]");
    await expect(cells.first()).toBeVisible({ timeout: 15_000 });
  });

  test("chat input is visible during game", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_chat_${suffix}`, "testpass123");

    const gameId = await createSoloGameApi(token);
    await page.goto(`/game/${gameId}`);

    const cells = page.locator("[data-row][data-col]");
    await expect(cells.first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByPlaceholder("Type a message...")).toBeVisible();
  });

  test("game page shows theme and difficulty labels", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_badge_${suffix}`, "testpass123");

    const gameId = await createSoloGameApi(token);
    await page.goto(`/game/${gameId}`);

    const cells = page.locator("[data-row][data-col]");
    await expect(cells.first()).toBeVisible({ timeout: 15_000 });

    // Theme + difficulty shown in header area
    await expect(page.locator("main").getByText("Science")).toBeVisible();
    await expect(page.locator("main").getByText("medium")).toBeVisible();
  });

  test("back to lobby button works", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_back_${suffix}`, "testpass123");

    const gameId = await createGameApi(token);
    await page.goto(`/game/${gameId}`);
    await expect(page.getByText("Waiting for players")).toBeVisible({ timeout: 10_000 });

    // Click the first button inside main (back arrow)
    await page.locator("main button").first().click();
    await expect(page).toHaveURL(/\/lobby/, { timeout: 10_000 });
  });
});
