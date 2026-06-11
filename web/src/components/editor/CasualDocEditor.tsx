/**
 * CasualDocEditor — Drive's mount for `.docx` files via the iframe
 * variant `<CasualEditorIframe>` from `@schnsrw/docx-js-editor@>=1.1.0`.
 *
 * Why the iframe variant (not the direct mount):
 *   - CSS isolation. Univer's design tokens + the docx editor's CSS
 *     no longer leak into Drive's tree.
 *   - React-runtime isolation. The SDK's React 19 instance ran into
 *     `LocaleService: Locale not initialized` when mounted alongside
 *     Drive's React tree — that crash goes away with the iframe.
 *   - `viewMode='preview'` hides the toolbar inside the iframe so the
 *     Preview modal renders JUST the rendered document canvas.
 *     `viewMode='editor'` shows the full toolbar for `/file/<id>`.
 *
 * The iframe is same-origin: its `src` resolves under Drive's own
 * domain (`${BASE_URL}embed/docs/embed.html?...`) — the embed runtime
 * is copied from `@schnsrw/docx-js-editor/embed/*` into Drive's
 * `public/embed/docs/` by `scripts/copy-embed.mjs` at prebuild time.
 */

import { useMemo } from "react";

import { CasualEditorIframe } from "@schnsrw/docx-js-editor";

import { type FileDto } from "../../api/client.ts";
import { DriveFileSource } from "../../file-source/DriveFileSource.ts";

export interface CasualDocEditorProps {
  file: FileDto;
  /** `preview` = no toolbar, just canvas (modal mount). `editor` =
   *  full editor chrome (fullscreen route). */
  mode?: "preview" | "editor";
}

export function CasualDocEditor({ file, mode = "preview" }: CasualDocEditorProps) {
  const fileSource = useMemo(() => new DriveFileSource(file), [file.id]);
  const embedBasePath = `${import.meta.env.BASE_URL}embed/docs`;

  return (
    <CasualEditorIframe
      fileSource={fileSource}
      docId={file.id}
      viewMode={mode}
      embedBasePath={embedBasePath}
      testId="casual-doc-editor"
    />
  );
}
