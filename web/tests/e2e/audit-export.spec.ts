import { expect, test } from "@playwright/test";
import { resetDemoState, signInDemo } from "./_helpers.ts";

test.beforeEach(async ({ page }) => {
  await resetDemoState(page);
  await signInDemo(page);
});

test("admin exports a downloadable audit report", async ({ page }) => {
  // Sidebar nav-row buttons are role=button with the label as text.
  await page.getByRole("button").filter({ hasText: /^Admin$/ }).first().click();
  await expect(page.getByRole("heading", { name: "Admin", exact: true })).toBeVisible();

  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByTestId("audit-export-button").click(),
  ]);
  expect(download.suggestedFilename()).toBe("audit-export.json");

  // The card confirms the export and points at offline verification.
  await expect(page.getByTestId("audit-export-note")).toContainText("verify-audit");
});
