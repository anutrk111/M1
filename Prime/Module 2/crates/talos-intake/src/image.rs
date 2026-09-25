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

/// Sniff magic, check header dimensions against config, then fully decode. Original
/// bytes are never written.
///
/// - Unrecognized magic, unreadable header, or failed decode (corrupt / truncated /
///   fake magic) → [`TalosError::Permanent`] (`RejectedPermanent`).
/// - Configured width / height / pixel-budget violations, whether seen in the header or
///   raised by the decoder's limits → [`TalosError::Validation`] (`RejectedValidation`).
///
/// The header is not trusted on its own: decode must succeed and yield the same
/// dimensions.
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

    let (hw, hh) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| TalosError::Permanent(format!("image header unreadable: {e}")))?;
    check_dimensions(hw, hh, config)?;

    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(config.max_width);
    limits.max_image_height = Some(config.max_height);
    // Up to 16-bit RGBA per pixel within the configured pixel budget.
    limits.max_alloc = Some(config.max_pixel_count.saturating_mul(8));
    reader.limits(limits);

    let dyn_img = reader.decode().map_err(classify_decode_error)?;

    let (w, h) = dyn_img.dimensions();
    if (w, h) != (hw, hh) {
        return Err(TalosError::Permanent(format!(
            "decoded dimensions {w}x{h} disagree with header {hw}x{hh}"
        )));
    }
    check_dimensions(w, h, config)?;
    Ok(content_type)
}

/// Decoder limits are set only from config, so a `Limits` error is a budget violation.
fn classify_decode_error(e: image::ImageError) -> TalosError {
    match e {
        image::ImageError::Limits(_) => {
            TalosError::Validation(format!("image exceeds configured decode limits: {e}"))
        }
        _ => TalosError::Permanent(format!("image decode failed: {e}")),
    }
}

fn check_dimensions(w: u32, h: u32, config: &IntakeConfig) -> Result<(), TalosError> {
    if w > config.max_width || h > config.max_height {
        return Err(TalosError::Validation(format!(
            "image dimensions {w}x{h} exceed max {}x{}",
            config.max_width, config.max_height
        )));
    }
    let pixels = u64::from(w).saturating_mul(u64::from(h));
    if pixels > config.max_pixel_count {
        return Err(TalosError::Validation(format!(
            "image pixel count {pixels} exceeds max {}",
            config.max_pixel_count
        )));
    }
    Ok(())
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

    fn encode(w: u32, h: u32, format: ImageFormat) -> Vec<u8> {
        let img = ImageBuffer::from_pixel(w, h, Rgb([90u8, 60, 30]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), format).unwrap();
        buf
    }

    fn cfg(max_width: u32, max_height: u32, max_pixel_count: u64) -> IntakeConfig {
        IntakeConfig {
            max_width,
            max_height,
            max_pixel_count,
            ..IntakeConfig::default()
        }
    }

    #[test]
    fn classification_matrix() {
        let jpeg = encode(40, 20, ImageFormat::Jpeg);
        let png = encode(40, 20, ImageFormat::Png);
        let mut corrupt = jpeg.clone();
        for b in corrupt.iter_mut().skip(4) {
            *b = 0x00;
        }
        let mut truncated = jpeg.clone();
        truncated.truncate(jpeg.len() * 2 / 3);
        let loose = cfg(8192, 8192, 25_000_000);

        let validation = |r: Result<&str, TalosError>| matches!(r, Err(TalosError::Validation(_)));
        let permanent = |r: Result<&str, TalosError>| matches!(r, Err(TalosError::Permanent(_)));

        assert_eq!(validate_image(&jpeg, &loose).unwrap(), "image/jpeg");
        assert_eq!(validate_image(&png, &loose).unwrap(), "image/png");
        assert!(permanent(validate_image(&corrupt, &loose)), "corrupt JPEG");
        assert!(
            permanent(validate_image(&truncated, &loose)),
            "truncated JPEG"
        );

        for bytes in [&jpeg, &png] {
            assert!(
                validation(validate_image(bytes, &cfg(39, 8192, 25_000_000))),
                "width"
            );
            assert!(
                validation(validate_image(bytes, &cfg(8192, 19, 25_000_000))),
                "height"
            );
            assert!(
                validation(validate_image(bytes, &cfg(8192, 8192, 799))),
                "pixels"
            );
            assert!(
                validate_image(bytes, &cfg(40, 20, 800)).is_ok(),
                "exact limits"
            );
        }
    }

    #[test]
    fn decoder_limit_errors_map_to_validation() {
        let png = encode(64, 64, ImageFormat::Png);
        let mut reader = ImageReader::with_format(Cursor::new(&png[..]), ImageFormat::Png);
        let mut limits = Limits::default();
        limits.max_alloc = Some(1024);
        reader.limits(limits);
        let err = reader.decode().unwrap_err();
        assert!(matches!(err, image::ImageError::Limits(_)), "{err:?}");
        assert!(matches!(
            classify_decode_error(err),
            TalosError::Validation(_)
        ));

        let mut truncated = png.clone();
        truncated.truncate(png.len() / 2);
        let err = ImageReader::with_format(Cursor::new(&truncated[..]), ImageFormat::Png)
            .decode()
            .unwrap_err();
        assert!(matches!(
            classify_decode_error(err),
            TalosError::Permanent(_)
        ));
    }

    #[test]
    fn sixteen_bit_png_within_pixel_budget_is_accepted() {
        let img = ImageBuffer::<image::Rgba<u16>, _>::from_pixel(64, 64, image::Rgba([1, 2, 3, 4]));
        let mut png = Vec::new();
        img.write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .unwrap();
        assert!(validate_image(&png, &cfg(8192, 8192, 64 * 64)).is_ok());
    }
}
