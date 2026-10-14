import { test, expect } from "@playwright/test";
import { registerUserApi, uniqueSuffix, waitForProfilePicker } from "./helpers";

test.describe("Login page", () => {
  test("shows server player picker and simple create form", async ({ page }) => {
    await page.goto("/login");
    await waitForProfilePicker(page);

    await expect(page.getByRole("heading", { name: "Pick a player" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Create a player" })).toBeVisible();
    await expect(page.getByLabel("Search Players")).toBeVisible();
    await expect(page.getByPlaceholder("Pick a player name")).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Player" })).toBeVisible();
  });

  test("creating a player from login redirects to lobby", async ({ page }) => {
    const suffix = uniqueSuffix();

    await page.goto("/login");
    await waitForProfilePicker(page);
    await page.getByPlaceholder("Pick a player name").fill(`e2e_login_${suffix}`);
    await page.getByRole("button", { name: "Create Player" }).click();

    await expect(page).toHaveURL(/\/lobby/, { timeout: 15_000 });
  });

  test("existing server player can be picked from the list", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_pick_${suffix}`;
    await registerUserApi(username, "testpass123");

    await page.goto("/login");
    await waitForProfilePicker(page);
    await expect(page.getByText(username, { exact: true })).toBeVisible({ timeout: 10_000 });
    await page
      .locator("div.rounded-2xl.border")
      .filter({ has: page.getByText(username, { exact: true }) })
      .getByRole("button", { name: "Play", exact: true })
      .click();

    await expect(page).toHaveURL(/\/lobby/, { timeout: 15_000 });
  });

  test("search can narrow the server player list", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_search_${suffix}`;
    await registerUserApi(username, "testpass123");

    await page.goto("/login");
    await waitForProfilePicker(page);
    await page.getByLabel("Search Players").fill(username.slice(-6));
    await expect(page.getByText(username, { exact: true })).toBeVisible({ timeout: 10_000 });
  });

  test("duplicate username shows guidance to pick it from the list", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_existing_${suffix}`;
    await registerUserApi(username, "testpass123");

    await page.goto("/login");
    await waitForProfilePicker(page);
    await page.getByPlaceholder("Pick a player name").fill(username);
    await page.getByRole("button", { name: "Create Player" }).click();

    await expect(page.getByText("That player already exists. Pick it from the list on the left.")).toBeVisible({
      timeout: 10_000,
    });
  });
});
