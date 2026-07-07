import { expect, test } from "@playwright/test";
import { resetDemoState, signInDemo } from "./_helpers.ts";

test.beforeEach(async ({ page }) => {
  await resetDemoState(page);
  await signInDemo(page);
});

const OPEN_PALETTE = process.platform === "darwin" ? "Meta+K" : "Control+K";
const VERIFY_CHORD = process.platform === "darwin" ? "Meta+Shift+V" : "Control+Shift+V";

// UI-M6 (gap 3): the command palette surfaces context-aware compliance
// ACTIONS once a document is the active context (the top document match).
test("command palette shows compliance ACTIONS for the active document", async ({ page }) => {
  await page.keyboard.press(OPEN_PALETTE);
  const input = page.getByPlaceholder(/Search documents, notes/);
  await expect(input).toBeVisible();
  await input.fill("planning");

  const palette = page.getByRole("dialog", { name: "Command palette" });
  // Group heading names the active document so the actions' scope is clear.
  await expect(palette.getByText(/Actions · Q2 planning\.xlsx/)).toBeVisible({ timeout: 3_000 });
  // All four compliance/registry commands appear, keyboard-driven.
  await expect(palette.getByRole("option", { name: /Verify chain/ })).toBeVisible();
  await expect(palette.getByRole("option", { name: /^Sign/ })).toBeVisible();
  await expect(palette.getByRole("option", { name: /Place legal hold/ })).toBeVisible();
  await expect(palette.getByRole("option", { name: /Export provenance bundle/ })).toBeVisible();
  // Verify chain carries its mono ⌘⇧V accelerator chip.
  await expect(palette.getByText("⌘⇧V")).toBeVisible();
});

test("selecting an ACTION routes to the document's verify + provenance surface", async ({ page }) => {
  await page.keyboard.press(OPEN_PALETTE);
  await page.getByPlaceholder(/Search documents, notes/).fill("planning");
  const palette = page.getByRole("dialog", { name: "Command palette" });
  await palette.getByRole("option", { name: /Export provenance bundle/ }).click();
  // Lands on the compliance surface for that document.
  await expect(page.getByTestId("version-history-page")).toBeVisible({ timeout: 5_000 });
  await expect(page).toHaveURL(/\/document\/[^/]+\/history/);
});

test("⌘⇧V verifies the active document's chain from the palette", async ({ page }) => {
  await page.keyboard.press(OPEN_PALETTE);
  await page.getByPlaceholder(/Search documents, notes/).fill("planning");
  await expect(
    page.getByRole("dialog", { name: "Command palette" }).getByText(/Actions ·/),
  ).toBeVisible({ timeout: 3_000 });
  await page.keyboard.press(VERIFY_CHORD);
  await expect(page.getByTestId("version-history-page")).toBeVisible({ timeout: 5_000 });
});
