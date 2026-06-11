#!/usr/bin/env node
/**
 * Copies the SDK iframe-embed runtime (embed.html + embed-runtime.*)
 * from each editor package's dist into Drive's public/embed/ tree so
 * the SPA can serve them same-origin. The `<CasualSheetsIframe>` and
 * `<CasualEditorIframe>` components default `embedBasePath` to
 * `/embed/sheets` and `/embed/docs` — these paths exist after this
 * script runs and after Vite copies public/ to dist/.
 *
 * Runs at prebuild time (see package.json's `prebuild` script). The
 * resulting files are NOT committed (see .gitignore); they regenerate
 * whenever the SDK deps update.
 */
import { cpSync, mkdirSync, existsSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const require_ = createRequire(import.meta.url);

const PACKAGES = [
  // [npm name, public/embed/<subdir>, exports-key we use to anchor to the embed/ dir]
  ["@schnsrw/casual-sheets", "sheets", "embed/embed.html"],
  ["@schnsrw/docx-js-editor", "docs", "embed/embed.html"],
];

let failed = false;

for (const [pkg, subdir, anchor] of PACKAGES) {
  try {
    // Both packages restrict the `exports` field — `package.json` isn't
    // exported. Instead, resolve a known export (embed.html) and walk
    // back to its containing directory.
    const anchorPath = require_.resolve(`${pkg}/${anchor}`);
    const srcEmbedDir = dirname(anchorPath);
    if (!existsSync(srcEmbedDir)) {
      console.error(`[copy-embed] ${pkg}: ${srcEmbedDir} doesn't exist`);
      failed = true;
      continue;
    }
    const dstDir = resolve(root, "public", "embed", subdir);
    rmSync(dstDir, { recursive: true, force: true });
    mkdirSync(dstDir, { recursive: true });
    cpSync(srcEmbedDir, dstDir, { recursive: true });

    // Patch the embed.html in place for two upstream packaging bugs:
    //   - sheet 0.5.0 references `embed-runtime.css` but the SDK doesn't
    //     ship one. Strip the stylesheet link so the browser doesn't
    //     log a 404 — the embed-runtime.js inlines its own styles.
    //   - doc 1.1.0 imports `./embed-runtime.js` but the dist actually
    //     ships `.mjs`. Rewrite the import to point at the real file.
    // Both are upstream issues; this keeps drive's runtime quiet
    // until the SDKs ship a corrected build.
    const htmlPath = resolve(dstDir, "embed.html");
    if (existsSync(htmlPath)) {
      const raw = readFileSync(htmlPath, "utf8");
      let patched = raw;
      const cssFile = resolve(dstDir, "embed-runtime.css");
      if (!existsSync(cssFile)) {
        patched = patched.replace(
          /\s*<link rel="stylesheet" href="\.\/embed-runtime\.css" \/>\s*/g,
          "\n    ",
        );
      }
      const jsFile = resolve(dstDir, "embed-runtime.js");
      const mjsFile = resolve(dstDir, "embed-runtime.mjs");
      if (!existsSync(jsFile) && existsSync(mjsFile)) {
        patched = patched.replace(/embed-runtime\.js/g, "embed-runtime.mjs");
      }
      if (patched !== raw) {
        writeFileSync(htmlPath, patched);
        console.log(`[copy-embed] ${pkg}: patched embed.html`);
      }
    }

    const copied = readdirSync(dstDir);
    console.log(`[copy-embed] ${pkg} → public/embed/${subdir}/  (${copied.length} files)`);
    for (const f of copied) console.log(`[copy-embed]   ${f}`);
  } catch (err) {
    console.error(`[copy-embed] ${pkg}: ${err instanceof Error ? err.message : err}`);
    failed = true;
  }
}

if (failed) {
  console.error("[copy-embed] one or more packages failed; aborting build");
  process.exit(1);
}
