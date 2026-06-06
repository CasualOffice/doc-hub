// One-shot screenshot script to inspect the current SPA. Not a real test.
import { chromium } from "@playwright/test";

const URL = process.env.URL ?? "http://127.0.0.1:18090/";
const OUT = process.env.OUT ?? "/tmp";

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 2,
});
const page = await ctx.newPage();

console.log(`fetching ${URL}`);
await page.goto(URL);
await page.waitForLoadState("networkidle");
await page.screenshot({ path: `${OUT}/cd-signin.png`, fullPage: false });
console.log("→ saved signin");

// Sign in
try {
  await page.fill('input[name="username"]', "admin", { timeout: 5_000 });
  await page.fill('input[name="password"]', "hunter2", { timeout: 5_000 });
  await page.click('button[type="submit"]', { timeout: 5_000 });
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(500);
  await page.screenshot({ path: `${OUT}/cd-shell.png`, fullPage: false });
  console.log("→ saved shell");
} catch (e) {
  console.log("signin step skipped:", e.message);
}

// Dump the actual computed body background + color for diagnosis
const body = await page.evaluate(() => {
  const cs = window.getComputedStyle(document.body);
  const root = window.getComputedStyle(document.documentElement);
  return {
    body_bg: cs.backgroundColor,
    body_color: cs.color,
    body_font: cs.fontFamily,
    root_paper: root.getPropertyValue("--paper"),
    root_ink: root.getPropertyValue("--ink"),
    root_font_sans: root.getPropertyValue("--font-sans"),
    html_data_theme: document.documentElement.getAttribute("data-theme"),
  };
});
console.log("body computed:", JSON.stringify(body, null, 2));

await browser.close();
