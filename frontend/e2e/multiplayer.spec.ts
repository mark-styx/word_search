import { test, expect } from "@playwright/test";
import {
  registerAndAuth,
  uniqueSuffix,
  createGameApi,
  joinGameApi,
  startGameApi,
  registerUserApi,
} from "./helpers";

test.describe("Multiplayer game flow", () => {
  test("two players can play a game together", async ({ browser }) => {
    // Create two browser contexts for two players
    const suffix = uniqueSuffix();
    const hostName = `e2e_host_${suffix}`;
    const guestName = `e2e_guest_${suffix}`;
    const password = "testpass123";

    // Register both users via API
    const hostToken = await registerUserApi(hostName, password);
    const guestToken = await registerUserApi(guestName, password);

    // Host creates game (medium grid is 12x12, fits all words safely)
    const gameId = await createGameApi(hostToken, "science", "medium");

    // Guest joins via API
    await joinGameApi(guestToken, gameId);

    // Start the game
    await startGameApi(hostToken, gameId);

    // Open host's game page
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await hostPage.goto("/");
    await hostPage.evaluate((t) => localStorage.setItem("access_token", t), hostToken);
    await hostPage.goto(`/game/${gameId}`);

    // Open guest's game page
    const guestCtx = await browser.newContext();
    const guestPage = await guestCtx.newPage();
    await guestPage.goto("/");
    await guestPage.evaluate((t) => localStorage.setItem("access_token", t), guestToken);
    await guestPage.goto(`/game/${gameId}`);

    // Both should see the game grid
    await expect(hostPage.getByText("Scores")).toBeVisible({ timeout: 15_000 });
    await expect(guestPage.getByText("Scores")).toBeVisible({ timeout: 15_000 });

    // Host sees their username in scores
    await expect(hostPage.getByText(hostName)).toBeVisible();
    // Guest sees their username in scores
    await expect(guestPage.getByText(guestName)).toBeVisible();

    // Both should see the chat input (proves WS connected and game UI rendered)
    await expect(hostPage.getByPlaceholder("Type a message...")).toBeVisible({ timeout: 10_000 });
    await expect(guestPage.getByPlaceholder("Type a message...")).toBeVisible({ timeout: 5_000 });

    // Cleanup
    await hostCtx.close();
    await guestCtx.close();
  });

  test("lobby shows waiting state and player list", async ({ browser }) => {
    const suffix = uniqueSuffix();
    const hostName = `e2e_wait_${suffix}`;
    const guestName = `e2e_waitg_${suffix}`;
    const password = "testpass123";

    const hostToken = await registerUserApi(hostName, password);
    const guestToken = await registerUserApi(guestName, password);

    const gameId = await createGameApi(hostToken);

    // Host opens game page (lobby state, not started)
    const hostCtx = await browser.newContext();
    const hostPage = await hostCtx.newPage();
    await hostPage.goto("/");
    await hostPage.evaluate((t) => localStorage.setItem("access_token", t), hostToken);

    await hostPage.goto(`/game/${gameId}`);
    await expect(hostPage.getByText("Waiting for players")).toBeVisible({ timeout: 10_000 });
    await expect(hostPage.getByText(hostName)).toBeVisible();

    // Guest joins via API
    await joinGameApi(guestToken, gameId);

    // Refresh host page to see guest
    await hostPage.reload();
    await expect(hostPage.getByText(guestName)).toBeVisible({ timeout: 10_000 });

    await hostCtx.close();
  });
});
