import { test, expect } from "@playwright/test";
import { registerUserApi, uniqueSuffix, waitForProfilePicker } from "./helpers";

test.describe("Registration page", () => {
  test("shows simplified player creation screen", async ({ page }) => {
    await page.goto("/register");
    await waitForProfilePicker(page);

    await expect(page.getByRole("heading", { name: "Pick a player" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Create a player" })).toBeVisible();
    await expect(page.getByLabel("Search Players")).toBeVisible();
    await expect(page.getByPlaceholder("Pick a player name")).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Player" })).toBeVisible();
  });

  test("creating a player from register redirects to lobby", async ({ page }) => {
    const suffix = uniqueSuffix();

    await page.goto("/register");
    await waitForProfilePicker(page);
    await page.getByPlaceholder("Pick a player name").fill(`e2e_register_${suffix}`);
    await page.getByRole("button", { name: "Create Player" }).click();

    await expect(page).toHaveURL(/\/lobby/, { timeout: 15_000 });
  });

  test("short usernames show an error", async ({ page }) => {
    await page.goto("/register");
    await waitForProfilePicker(page);

    await page.getByPlaceholder("Pick a player name").fill("ab");
    await page.getByRole("button", { name: "Create Player" }).click();

    await expect(page.getByText("Username must be at least 3 characters")).toBeVisible();
  });

  test("duplicate usernames show an error", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_dup_${suffix}`;
    await registerUserApi(username, "testpass123");

    await page.goto("/register");
    await waitForProfilePicker(page);
    await page.getByPlaceholder("Pick a player name").fill(username);
    await page.getByRole("button", { name: "Create Player" }).click();

    await expect(page.getByText("That player already exists. Pick it from the list on the left.")).toBeVisible({
      timeout: 10_000,
    });
  });
});
