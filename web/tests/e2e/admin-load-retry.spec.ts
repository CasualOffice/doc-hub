import { expect, test } from "@playwright/test";
import { resetDemoState, signInDemo } from "./_helpers.ts";

test.beforeEach(async ({ page }) => {
  await resetDemoState(page);
});

test("a failed admin load shows a retry that recovers", async ({ page }) => {
  // Force the admin overview to fail. addInitScript runs after resetDemoState's
  // clear-init-script, so the flag survives the wipe and the demo shim throws
  // 503 for /api/admin/system (and the Files/Activity listings — harmless here,
  // we drive straight to the Admin tab).
  await page.addInitScript(() => {
    try {
      window.localStorage.setItem("cd-demo-force-error", "1");
    } catch {
      /* ignored */
    }
  });

  await signInDemo(page);

  // Open the Admin tab — its one-shot load fails.
  await page.getByRole("button").filter({ hasText: /^Admin$/ }).first().click();
  await expect(page.getByRole("heading", { name: "Admin", exact: true })).toBeVisible();

  // The error surface offers recovery instead of a dead end.
  const retry = page.getByRole("button", { name: "Try again" });
  await expect(retry).toBeVisible({ timeout: 10_000 });

  // Clear the flag and retry in place — no reload.
  await page.evaluate(() => {
    try {
      window.localStorage.removeItem("cd-demo-force-error");
    } catch {
      /* ignored */
    }
  });
  await retry.click();

  // Recovery: the admin cards render (the System card is always present).
  await expect(page.getByRole("button", { name: "Try again" })).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "System", exact: true })).toBeVisible({
    timeout: 5_000,
  });
});
