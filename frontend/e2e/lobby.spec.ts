import { test, expect } from "@playwright/test";
import {
  registerAndAuth,
  uniqueSuffix,
  createGameApi,
  listWordBanksApi,
  createGameWithWordBankApi,
  startGameApi,
} from "./helpers";

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
      main.getByText("No active games right now").or(main.getByText(/Host:/).first())
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

    // Should have word bank and difficulty selects
    await expect(page.getByLabel("Word Bank")).toBeVisible();
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

    // Click the game card (div[role="button"]) with this user as host
    const username = `e2e_join_${suffix}`;
    const gameCard = page.getByRole("button", { name: new RegExp(`Host: ${username}`) });
    await expect(gameCard).toBeVisible({ timeout: 10_000 });
    await gameCard.click();

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

  test("can generate a custom word bank and use it in a new game", async ({ page }) => {
    const suffix = uniqueSuffix();
    await registerAndAuth(page, `e2e_generate_${suffix}`, "testpass123");

    await page.goto("/lobby");
    await page.getByRole("button", { name: /Generate Word Bank/ }).click();

    await expect(page.getByRole("heading", { name: "Generate Word Bank" })).toBeVisible();
    await page.getByLabel("What should the bank be about?").fill("ocean animals");
    await page.getByRole("button", { name: "Generate", exact: true }).click();

    await expect(page.getByText("Word bank request sent. I’ll add it to the list when it finishes.")).toBeVisible();
    await expect(page.getByText('"Ocean Pals" is ready to use.')).toBeVisible({ timeout: 15_000 });

    await expect(page.getByRole("heading", { name: "Custom Word Banks" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Ocean Pals" }).first()).toBeVisible();
    await expect(page.getByText("Coverage 0%.").first()).toBeVisible();

    await expect(page.getByRole("heading", { name: "Create Game" })).toBeVisible();
    await expect(page.getByLabel("Word Bank")).toHaveValue(/custom:/);
    await page.getByRole("button", { name: "Create", exact: true }).click();

    await expect(page).toHaveURL(/\/game\//, { timeout: 15_000 });
    await expect(page.locator("main").getByText("Ocean Pals")).toBeVisible();
  });

  test("custom word bank coverage increases and can trigger a refill", async ({ page }) => {
    const suffix = uniqueSuffix();
    const token = await registerAndAuth(page, `e2e_refill_${suffix}`, "testpass123");
    let bankId = "";

    await page.goto("/lobby");
    await page.getByRole("button", { name: /Generate Word Bank/ }).click();
    await page.getByLabel("What should the bank be about?").fill("friendly ocean adventure");
    await page.getByRole("button", { name: "Generate", exact: true }).click();
    await expect(page.getByText('"Ocean Pals" is ready to use.')).toBeVisible({ timeout: 15_000 });

    await expect
      .poll(async () => {
        const banks = await listWordBanksApi(token);
        bankId =
          banks.find((bank: { label: string; source: string }) => bank.label === "Ocean Pals" && bank.source === "custom")?.id ?? "";
        return bankId;
      }, { timeout: 15_000 })
      .not.toBe("");

    for (let i = 0; i < 4; i += 1) {
      const gameId = await createGameWithWordBankApi(token, bankId, "easy");
      await startGameApi(token, gameId);
    }

    await expect
      .poll(async () => {
        const banks = await listWordBanksApi(token);
        const bank = banks.find((entry: { id: string }) => entry.id === bankId);
        return bank
          ? {
              used: bank.used_words_count,
              total: bank.total_words,
              coverage: bank.coverage_percent,
              extensionState: bank.extension_state,
            }
          : null;
      }, { timeout: 20_000 })
      .toMatchObject({
        used: expect.any(Number),
        total: expect.any(Number),
        coverage: expect.any(Number),
        extensionState: expect.any(String),
      });

    const banks = await listWordBanksApi(token);
    const bank = banks.find((entry: { id: string }) => entry.id === bankId);
    expect(bank).toBeTruthy();
    expect(bank.used_words_count).toBeGreaterThan(0);
    expect(bank.coverage_percent).toBeGreaterThan(0);
    expect(bank.total_words).toBeGreaterThanOrEqual(48);

    await page.goto("/lobby");
    await expect(page.getByRole("heading", { name: "Ocean Pals" }).first()).toBeVisible();
    await expect(page.getByText(/unique words used/).first()).toBeVisible();
  });
});
