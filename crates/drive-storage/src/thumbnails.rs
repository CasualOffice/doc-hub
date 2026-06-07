//! Server-side thumbnail decoder. Pipeline §5.4.
//! Spec: docs/research/11-server-thumbnails.md.
//!
//! v0 ships an in-process IMAGE-ONLY worker (`image` crate). PDF + video
//! decoders are explicitly NOT in-process — they need a sandboxed
//! subprocess per the security brief, and that lands in v0.2.
//!
//! Concrete impl for now; once a second worker (subprocess wrapper)
//! exists we'll introduce a `ThumbnailWorker` trait.

use bytes::Bytes;
use std::io::Cursor;

/// Pre-classified hint so the worker doesn't have to re-sniff. We give it
/// the broad bucket and the byte stream — the worker decides whether it
/// can handle the kind in-process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailKind {
    Image,
    /// PDF rendering ships in v0.2. The trait accepts the kind so callers
    /// don't have to special-case "what's left over" today.
    Pdf,
    /// Video frame extraction ships in v0.2 for the same reason.
    Video,
}

#[derive(Debug, thiserror::Error)]
pub enum ThumbnailError {
    #[error("unsupported kind: {0:?}")]
    Unsupported(ThumbnailKind),
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("encode failed: {0}")]
    Encode(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    /// Crop to a square — used for `small` and `medium` (grid cells).
    Cover,
    /// Keep the full image, letterboxed inside the square — used for
    /// `large` (preview pane).
    Contain,
}

/// The 3 canonical sizes shipped to the SPA. Keeping them as an enum
/// (rather than `u32`) prevents callers from minting arbitrary sizes
/// that would balloon the bucket footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThumbSize {
    Small,
    Medium,
    Large,
}

impl ThumbSize {
    #[must_use]
    pub fn px(self) -> u32 {
        match self {
            Self::Small => 96,
            Self::Medium => 256,
            Self::Large => 1024,
        }
    }
    #[must_use]
    pub fn fit_mode(self) -> FitMode {
        match self {
            Self::Small | Self::Medium => FitMode::Cover,
            Self::Large => FitMode::Contain,
        }
    }
    #[must_use]
    pub fn key_suffix(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
    pub fn all() -> [ThumbSize; 3] {
        [Self::Small, Self::Medium, Self::Large]
    }
    /// Storage key for a given file id + size.
    /// `thumbs/{ulid}/{size}.png` — matches the spec.
    #[must_use]
    pub fn key_for(self, file_id: &str) -> String {
        format!("thumbs/{file_id}/{}.png", self.key_suffix())
    }
}

/// In-process, image-only worker. Safe for the `image` crate's PNG /
/// JPEG / WebP / GIF / BMP decoders (vetted; bounded memory at 50 MP).
/// Refuses PDF + video → callers see `Unsupported` and the file's
/// `thumbs_state` flips to `unsupported`.
#[derive(Debug, Default, Clone, Copy)]
pub struct ImageOnlyWorker;

impl ImageOnlyWorker {
    /// Decode `bytes` (already classified) and emit a PNG at the
    /// requested target dimension. `size_px` is interpreted as
    /// fit-cover-square for `small`/`medium` and fit-contain for `large`
    /// (see `ThumbSize::fit_mode`).
    pub async fn generate(
        &self,
        kind: ThumbnailKind,
        bytes: Bytes,
        size_px: u32,
        fit: FitMode,
    ) -> Result<Vec<u8>, ThumbnailError> {
        if !matches!(kind, ThumbnailKind::Image) {
            return Err(ThumbnailError::Unsupported(kind));
        }
        // Heavy work — push it onto a blocking thread so we don't stall
        // the tokio scheduler.
        let png = tokio::task::spawn_blocking(move || render_image(bytes, size_px, fit))
            .await
            .map_err(|e| ThumbnailError::Decode(format!("worker panicked: {e}")))??;
        Ok(png)
    }
}

fn render_image(bytes: Bytes, size_px: u32, fit: FitMode) -> Result<Vec<u8>, ThumbnailError> {
    let img = image::ImageReader::new(Cursor::new(bytes.as_ref()))
        .with_guessed_format()
        .map_err(|e| ThumbnailError::Decode(format!("guess format: {e}")))?
        .decode()
        .map_err(|e| ThumbnailError::Decode(format!("decode: {e}")))?;

    let resized = match fit {
        FitMode::Cover => image::imageops::resize(
            &img.to_rgba8(),
            size_px,
            size_px,
            image::imageops::FilterType::Lanczos3,
        ),
        FitMode::Contain => {
            // Letterbox into a transparent square.
            let scaled = img.resize(size_px, size_px, image::imageops::FilterType::Lanczos3);
            let mut canvas =
                image::RgbaImage::from_pixel(size_px, size_px, image::Rgba([0, 0, 0, 0]));
            let (w, h) = (scaled.width(), scaled.height());
            let x = (size_px.saturating_sub(w)) / 2;
            let y = (size_px.saturating_sub(h)) / 2;
            image::imageops::overlay(&mut canvas, &scaled.to_rgba8(), x as i64, y as i64);
            canvas
        }
    };

    let mut out = Vec::with_capacity(64 * 1024);
    image::DynamicImage::ImageRgba8(resized)
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| ThumbnailError::Encode(format!("png write: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_png() -> Bytes {
        // 4×4 solid-red PNG built in-test so we don't ship fixtures.
        let img = image::RgbImage::from_pixel(4, 4, image::Rgb([255, 0, 0]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        Bytes::from(buf)
    }

    #[tokio::test]
    async fn image_worker_decodes_png() {
        let w = ImageOnlyWorker;
        let out = w
            .generate(ThumbnailKind::Image, tiny_png(), 96, FitMode::Cover)
            .await
            .unwrap();
        assert!(out.starts_with(&[0x89, b'P', b'N', b'G']), "not a PNG");
    }

    #[tokio::test]
    async fn image_worker_refuses_pdf() {
        let w = ImageOnlyWorker;
        let err = w
            .generate(ThumbnailKind::Pdf, tiny_png(), 96, FitMode::Cover)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ThumbnailError::Unsupported(ThumbnailKind::Pdf)
        ));
    }

    #[tokio::test]
    async fn image_worker_refuses_video() {
        let w = ImageOnlyWorker;
        let err = w
            .generate(ThumbnailKind::Video, tiny_png(), 96, FitMode::Cover)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            ThumbnailError::Unsupported(ThumbnailKind::Video)
        ));
    }

    #[tokio::test]
    async fn cover_returns_square() {
        let w = ImageOnlyWorker;
        let bytes = w
            .generate(ThumbnailKind::Image, tiny_png(), 96, FitMode::Cover)
            .await
            .unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!(decoded.width(), 96);
        assert_eq!(decoded.height(), 96);
    }

    #[tokio::test]
    async fn contain_letterboxes_into_square() {
        let w = ImageOnlyWorker;
        // Wide source so Contain has to letterbox.
        let img = image::RgbaImage::from_pixel(8, 2, image::Rgba([0, 255, 0, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        let bytes = w
            .generate(ThumbnailKind::Image, Bytes::from(buf), 64, FitMode::Contain)
            .await
            .unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!(decoded.width(), 64);
        assert_eq!(decoded.height(), 64);
    }

    #[test]
    fn thumb_size_key_matches_spec() {
        assert_eq!(ThumbSize::Small.key_for("01ABC"), "thumbs/01ABC/small.png");
        assert_eq!(
            ThumbSize::Medium.key_for("01ABC"),
            "thumbs/01ABC/medium.png"
        );
        assert_eq!(ThumbSize::Large.key_for("01ABC"), "thumbs/01ABC/large.png");
    }
}
