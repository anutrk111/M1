//! Magic sniff + decode validation. Does **not** re-encode or mutate bytes.

use crate::config::IntakeConfig;
use image::{GenericImageView, ImageFormat, ImageReader, Limits};
use std::io::Cursor;
use talos_core::TalosError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageKind {
    Jpeg,
    Png,
}

pub fn sniff_image(bytes: &[u8]) -> Option<(ImageKind, &'static str)> {
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some((ImageKind::Jpeg, "image/jpeg"));
    }
    if bytes.len() >= 8 && bytes[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some((ImageKind::Png, "image/png"));
    }
    None
}

/// Sniff magic, then decode with config pixel limits. Original bytes are never written.
///
/// Truncated / fake-magic payloads that fail decode map to
/// [`TalosError::Permanent`] (caller records `RejectedPermanent`).
/// Dimension / pixel-budget violations map to [`TalosError::Validation`].
pub fn validate_image(bytes: &[u8], config: &IntakeConfig) -> Result<&'static str, TalosError> {
    let Some((kind, content_type)) = sniff_image(bytes) else {
        return Err(TalosError::Permanent(
            "corrupt or unrecognized image magic".into(),
        ));
    };

    let format = match kind {
        ImageKind::Jpeg => ImageFormat::Jpeg,
        ImageKind::Png => ImageFormat::Png,
    };

    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(config.max_width);
    limits.max_image_height = Some(config.max_height);
    // Rough RGBA budget aligned with max_pixel_count.
    limits.max_alloc = Some(config.max_pixel_count.saturating_mul(4));
    reader.limits(limits);

    let dyn_img = reader
        .decode()
        .map_err(|e| TalosError::Permanent(format!("image decode failed: {e}")))?;

    let (w, h) = dyn_img.dimensions();
    let pixels = u64::from(w).saturating_mul(u64::from(h));
    if w > config.max_width || h > config.max_height {
        return Err(TalosError::Validation(format!(
            "image dimensions {w}x{h} exceed max {}x{}",
            config.max_width, config.max_height
        )));
    }
    if pixels > config.max_pixel_count {
        return Err(TalosError::Validation(format!(
            "image pixel count {pixels} exceeds max {}",
            config.max_pixel_count
        )));
    }

    Ok(content_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, ImageFormat, Rgb};

    fn encode_tiny_jpeg() -> Vec<u8> {
        let img = ImageBuffer::from_pixel(8, 8, Rgb([10u8, 20, 30]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Jpeg)
            .unwrap();
        buf
    }

    #[test]
    fn jpeg_magic() {
        let b = encode_tiny_jpeg();
        assert_eq!(sniff_image(&b).unwrap().1, "image/jpeg");
    }

    #[test]
    fn png_magic() {
        let b = [0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 1];
        assert_eq!(sniff_image(&b).unwrap().1, "image/png");
    }

    #[test]
    fn reject_garbage() {
        assert!(sniff_image(b"not an image").is_none());
    }

    #[test]
    fn validate_real_jpeg_ok() {
        let cfg = IntakeConfig::default();
        let b = encode_tiny_jpeg();
        assert_eq!(validate_image(&b, &cfg).unwrap(), "image/jpeg");
    }

    #[test]
    fn fake_magic_rejected() {
        let cfg = IntakeConfig::default();
        let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0];
        b.extend_from_slice(b"not a real jpeg body");
        let err = validate_image(&b, &cfg).unwrap_err();
        assert!(matches!(err, TalosError::Permanent(_)));
    }

    #[test]
    fn truncated_rejected() {
        let cfg = IntakeConfig::default();
        let mut b = encode_tiny_jpeg();
        b.truncate(b.len() / 2);
        let err = validate_image(&b, &cfg).unwrap_err();
        assert!(matches!(err, TalosError::Permanent(_)));
    }
}
