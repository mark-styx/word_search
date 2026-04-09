import { test, expect } from "@playwright/test";
import { registerUserApi, uniqueSuffix } from "./helpers";

test.describe("Login page", () => {
  test("shows login form", async ({ page }) => {
    await page.goto("/login");

    await expect(page.getByRole("heading", { name: "Welcome back" })).toBeVisible();
    await expect(page.getByLabel("Username")).toBeVisible();
    await expect(page.getByLabel("Password")).toBeVisible();
    await expect(page.getByRole("button", { name: "Sign In" })).toBeVisible();
    await expect(page.getByRole("link", { name: "Sign up" })).toBeVisible();
  });

  test("can navigate to register from login", async ({ page }) => {
    await page.goto("/login");
    await page.getByRole("link", { name: "Sign up" }).click();
    await expect(page).toHaveURL(/\/register/);
  });

  test("login with nonexistent user shows error", async ({ page }) => {
    await page.goto("/login");

    await page.getByPlaceholder("Enter your username").fill("nonexistent_user_xyz");
    await page.getByPlaceholder("Enter your password").fill("password123");
    await page.getByRole("button", { name: "Sign In" }).click();

    // Backend returns plain text "Invalid credentials", frontend shows "HTTP 401"
    await expect(page.locator(".text-error")).toBeVisible({ timeout: 10_000 });
  });

  test("login with wrong password shows error", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_lg_${suffix}`;
    await registerUserApi(username, "correctpass123");

    await page.goto("/login");
    await page.getByPlaceholder("Enter your username").fill(username);
    await page.getByPlaceholder("Enter your password").fill("wrongpass456");
    await page.getByRole("button", { name: "Sign In" }).click();

    await expect(page.locator(".text-error")).toBeVisible({ timeout: 10_000 });
  });

  test("successful login redirects to lobby", async ({ page }) => {
    const suffix = uniqueSuffix();
    const username = `e2e_login_${suffix}`;
    const password = "testpass123";
    await registerUserApi(username, password);

    await page.goto("/login");
    await page.getByPlaceholder("Enter your username").fill(username);
    await page.getByPlaceholder("Enter your password").fill(password);
    await page.getByRole("button", { name: "Sign In" }).click();

    await expect(page).toHaveURL(/\/lobby/, { timeout: 15_000 });
  });
});
