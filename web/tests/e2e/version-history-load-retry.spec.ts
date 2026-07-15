import { expect, test } from "@playwright/test";
import { resetDemoState, signInDemo } from "./_helpers.ts";

test.beforeEach(async ({ page }) => {
  await resetDemoState(page);
});

test("a failed version-history load shows a retry that recovers", async ({ page }) => {
  await page.addInitScript(() => {
    try {
      window.localStorage.setItem("cd-demo-force-error", "1");
    } catch {
      /* ignored */
    }
  });

  await signInDemo(page);

  // Client-side nav to the version-history route (a full reload would wipe the
  // demo session via resetDemoState's clear-init-script). App re-reads the
  // path on popstate.
  await page.evaluate(() => {
    window.history.pushState({}, "", "/document/f_quarter/history");
    window.dispatchEvent(new PopStateEvent("popstate"));
  });

  // The cold getFile fails → error surface with a retry, not a dead end.
  await expect(page.getByText(/Couldn't open version history/)).toBeVisible({ timeout: 10_000 });
  const retry = page.getByRole("button", { name: "Try again" });
  await expect(retry).toBeVisible();

  await page.evaluate(() => {
    try {
      window.localStorage.removeItem("cd-demo-force-error");
    } catch {
      /* ignored */
    }
  });
  await retry.click();

  // Recovery: the document is named (getFile resolved) and the error is gone.
  await expect(page.getByText(/Couldn't open version history/)).toHaveCount(0);
  await expect(page.getByText("Q2 planning.xlsx").first()).toBeVisible({ timeout: 5_000 });
});
