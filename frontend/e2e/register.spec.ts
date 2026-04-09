import { test, expect } from "@playwright/test";
import { registerUserApi, uniqueSuffix } from "./helpers";

test.describe("Registration page", () => {
  test("shows registration form", async ({ page }) => {
    await page.goto("/register");

    // Header is hidden on register page
    await expect(page.getByRole("heading", { name: "Create account" })).toBeVisible();
    await expect(page.getByPlaceholder("Choose a username")).toBeVisible();
    await expect(page.getByPlaceholder("At least 6 characters")).toBeVisible();
    await expect(page.getByPlaceholder("Repeat your password")).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Account" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Sign in" })).toBeVisible();
  });

  test("can navigate to login from register", async ({ page }) => {
    await page.goto("/register");
    await page.getByRole("link", { name: "Sign in" }).click();
    await expect(page).toHaveURL(/\/login/);
  });

  test("registering with short username shows error", async ({ page }) => {
    await page.goto("/register");

    await page.getByPlaceholder("Choose a username").fill("ab");
    await page.getByPlaceholder("At least 6 characters").fill("password123");
    await page.getByPlaceholder("Repeat your password").fill("password123");
    await page.getByRole("button", { name: "Create Account" }).click();

    await expect(page.getByText("Username must be at least 3 characters")).toBeVisible();
  });

  test("registering with short password shows error", async ({ page }) => {
    await page.goto("/register");

    const suffix = uniqueSuffix();
    await page.getByPlaceholder("Choose a username").fill(`user${suffix}`);
    await page.getByPlaceholder("At least 6 characters").fill("12345");
    await page.getByPlaceholder("Repeat your password").fill("12345");
    await page.getByRole("button", { name: "Create Account" }).click();

    await expect(page.getByText("Password must be at least 6 characters")).toBeVisible();
  });

  test("registering with mismatched passwords shows error", async ({ page }) => {
    await page.goto("/register");

    const suffix = uniqueSuffix();
    await page.getByPlaceholder("Choose a username").fill(`user${suffix}`);
    await page.getByPlaceholder("At least 6 characters").fill("password123");
    await page.getByPlaceholder("Repeat your password").fill("different456");
    await page.getByRole("button", { name: "Create Account" }).click();

    await expect(page.getByText("Passwords do not match")).toBeVisible();
  });

  test("successful registration redirects to lobby", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_${suffix}`;
    const password = "testpass123";

    await page.goto("/register");
    await page.getByPlaceholder("Choose a username").fill(username);
    await page.getByPlaceholder("At least 6 characters").fill(password);
    await page.getByPlaceholder("Repeat your password").fill(password);
    await page.getByRole("button", { name: "Create Account" }).click();

    await expect(page).toHaveURL(/\/lobby/, { timeout: 15_000 });
  });

  test("registering duplicate username shows error", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_dup_${suffix}`;
    const password = "testpass123";

    // Register via API first
    await registerUserApi(username, password);

    // Try to register same username via UI
    await page.goto("/register");
    await page.getByPlaceholder("Choose a username").fill(username);
    await page.getByPlaceholder("At least 6 characters").fill(password);
    await page.getByPlaceholder("Repeat your password").fill(password);
    await page.getByRole("button", { name: "Create Account" }).click();

    // Backend returns "Username already taken" or similar
    await expect(page.locator(".text-error")).toBeVisible({ timeout: 10_000 });
  });
});
