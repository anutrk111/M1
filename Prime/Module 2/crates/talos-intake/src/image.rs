/// Lightweight magic-byte checks. Does not re-encode or mutate bytes.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_magic() {
        let b = [0xFFu8, 0xD8, 0xFF, 0xE0, 0, 0];
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
}
