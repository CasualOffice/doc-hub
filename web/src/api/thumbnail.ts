// Client-side thumbnail generator. Spec: docs/ux/07-preview-surface.md
// + pipeline §5.2. Produces a small data URI for image uploads which the
// server stores on the file row and surfaces back in list responses.
//
// Browser-only — uses <canvas> + URL.createObjectURL. Returns null when
// the file isn't an image, the browser refuses to decode it, or the
// output blows past the size cap.

/** Target square dimension in CSS pixels. 192 covers both list-row
 * (30 px) and grid-card (130 px) at 2× DPR. */
const TARGET = 192;

/** Hard cap matching the server's THUMBNAIL_MAX_BYTES. Anything bigger
 * is dropped server-side anyway. */
const MAX_BYTES = 64 * 1024;

const SUPPORTED_IMAGE = /^image\/(png|jpe?g|gif|webp|avif|bmp)$/;
const SUPPORTED_VIDEO = /^video\/(mp4|webm|quicktime|ogg)$/;

/** Returns a `data:image/*;base64,…` URI or `null` if not applicable.
 *
 * Image inputs decode via createImageBitmap → canvas. Video inputs load
 * into an offscreen `<video>`, seek to a frame ~10% into the clip
 * (avoids the all-black first frame many encoders produce), and draw
 * the current frame to canvas. Same encoding pipeline + size cap
 * downstream — server-side `THUMBNAIL_MAX_BYTES` still gates final
 * acceptance, so this is purely best-effort. */
export async function generateThumbnail(file: File): Promise<string | null> {
  if (typeof window === "undefined") return null;
  if (SUPPORTED_VIDEO.test(file.type)) return generateVideoPoster(file);
  if (!SUPPORTED_IMAGE.test(file.type)) return null;

  let bitmap: ImageBitmap | null = null;
  let url: string | null = null;

  try {
    if (typeof createImageBitmap === "function") {
      bitmap = await createImageBitmap(file);
    } else {
      url = URL.createObjectURL(file);
      await loadImage(url);
    }
  } catch {
    if (url) URL.revokeObjectURL(url);
    return null;
  }

  const w = bitmap?.width ?? 0;
  const h = bitmap?.height ?? 0;
  if (w === 0 || h === 0) {
    if (url) URL.revokeObjectURL(url);
    return null;
  }

  const scale = Math.min(TARGET / w, TARGET / h, 1); // never upscale
  const outW = Math.max(1, Math.round(w * scale));
  const outH = Math.max(1, Math.round(h * scale));

  const canvas = document.createElement("canvas");
  canvas.width = outW;
  canvas.height = outH;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    if (url) URL.revokeObjectURL(url);
    return null;
  }
  ctx.imageSmoothingQuality = "high";
  if (bitmap) {
    ctx.drawImage(bitmap, 0, 0, outW, outH);
  } else if (url) {
    const img = await loadImage(url);
    ctx.drawImage(img, 0, 0, outW, outH);
  }
  if (url) URL.revokeObjectURL(url);

  // Try WebP first (best ratio at this size); fall back to JPEG, then PNG.
  const candidates = [
    () => canvas.toDataURL("image/webp", 0.82),
    () => canvas.toDataURL("image/jpeg", 0.82),
    () => canvas.toDataURL("image/png"),
  ];
  for (const make of candidates) {
    let uri: string;
    try {
      uri = make();
    } catch {
      continue;
    }
    if (uri && uri.length <= MAX_BYTES && uri.startsWith("data:image/")) return uri;
  }
  return null;
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("image decode failed"));
    img.src = src;
  });
}

/** Pipeline §5.3 — first-frame video poster.
 *
 * Loads the file into a `<video>` element (display:none), waits for
 * `loadedmetadata`, seeks to 10% of duration (or 0 for clips < 1s),
 * then captures the painted frame to a canvas. Encodes the result the
 * same way as the image path. Aborts after 5s if the browser can't
 * decode the codec or `seeked` never fires. */
async function generateVideoPoster(file: File): Promise<string | null> {
  const url = URL.createObjectURL(file);
  const video = document.createElement("video");
  video.preload = "auto";
  video.muted = true;
  video.playsInline = true;
  video.crossOrigin = "anonymous"; // tainted canvas otherwise on some codecs
  video.src = url;

  const cleanup = () => {
    URL.revokeObjectURL(url);
    video.removeAttribute("src");
    video.load();
  };

  const ready = new Promise<HTMLVideoElement>((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error("video metadata timeout")), 5_000);
    video.addEventListener(
      "loadedmetadata",
      () => {
        window.clearTimeout(timer);
        resolve(video);
      },
      { once: true },
    );
    video.addEventListener(
      "error",
      () => {
        window.clearTimeout(timer);
        reject(new Error("video decode failed"));
      },
      { once: true },
    );
  });

  let meta: HTMLVideoElement;
  try {
    meta = await ready;
  } catch {
    cleanup();
    return null;
  }

  const dur = Number.isFinite(meta.duration) ? meta.duration : 0;
  const targetTime = dur > 1 ? Math.min(dur * 0.1, 5) : 0;

  const seeked = new Promise<void>((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error("video seek timeout")), 5_000);
    video.addEventListener(
      "seeked",
      () => {
        window.clearTimeout(timer);
        resolve();
      },
      { once: true },
    );
    video.addEventListener(
      "error",
      () => {
        window.clearTimeout(timer);
        reject(new Error("video seek failed"));
      },
      { once: true },
    );
  });
  try {
    video.currentTime = targetTime;
    await seeked;
  } catch {
    cleanup();
    return null;
  }

  const w = video.videoWidth;
  const h = video.videoHeight;
  if (!w || !h) {
    cleanup();
    return null;
  }
  const scale = Math.min(TARGET / w, TARGET / h, 1);
  const outW = Math.max(1, Math.round(w * scale));
  const outH = Math.max(1, Math.round(h * scale));

  const canvas = document.createElement("canvas");
  canvas.width = outW;
  canvas.height = outH;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    cleanup();
    return null;
  }
  ctx.imageSmoothingQuality = "high";

  try {
    ctx.drawImage(video, 0, 0, outW, outH);
  } catch {
    // SecurityError when the source is a CORS-tainted blob URL on some
    // codecs — fall back to no poster.
    cleanup();
    return null;
  }
  cleanup();

  const candidates = [
    () => canvas.toDataURL("image/webp", 0.78),
    () => canvas.toDataURL("image/jpeg", 0.78),
    () => canvas.toDataURL("image/png"),
  ];
  for (const make of candidates) {
    let uri: string;
    try {
      uri = make();
    } catch {
      continue;
    }
    if (uri && uri.length <= MAX_BYTES && uri.startsWith("data:image/")) return uri;
  }
  return null;
}
