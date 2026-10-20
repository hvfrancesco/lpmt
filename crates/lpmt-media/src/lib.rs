//! Media decoding for LPMT.
//!
//! Images are always available. Video decoding is compiled in with the
//! `video` feature and decodes into tightly packed RGBA frames on a worker
//! thread; the renderer uploads those frames as GPU textures. Capture and
//! shared textures will follow the same CPU-to-GPU path.

use std::path::Path;

use anyhow::{Context, Result};

#[cfg(feature = "video")]
mod video;
#[cfg(feature = "video")]
pub use video::{PlaybackState, VideoFrame, VideoInfo, VideoPlayer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    /// Tightly packed RGBA8 pixels, top-left origin.
    pub rgba: Vec<u8>,
}

impl DecodedImage {
    pub fn from_rgba8(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self> {
        let expected = width as usize * height as usize * 4;
        anyhow::ensure!(
            rgba.len() == expected,
            "expected {expected} RGBA bytes for {width}x{height}, got {}",
            rgba.len()
        );
        Ok(Self {
            width,
            height,
            rgba,
        })
    }
}

pub fn load_image(path: impl AsRef<Path>) -> Result<DecodedImage> {
    let path = path.as_ref();
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read image {}", path.display()))?;
    decode_image(&bytes).with_context(|| format!("failed to decode image {}", path.display()))
}

pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage> {
    let image = image::load_from_memory(bytes).context("unsupported or corrupt image")?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    DecodedImage::from_rgba8(width, height, rgba.into_raw())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, Rgba, RgbaImage};

    use super::*;

    #[test]
    fn decodes_png_to_rgba8() {
        let image = RgbaImage::from_pixel(2, 3, Rgba([12, 34, 56, 255]));
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, ImageFormat::Png).unwrap();

        let decoded = decode_image(png.get_ref()).unwrap();
        assert_eq!(decoded.width, 2);
        assert_eq!(decoded.height, 3);
        assert_eq!(decoded.rgba.len(), 2 * 3 * 4);
        assert_eq!(&decoded.rgba[..4], &[12, 34, 56, 255]);
    }

    #[test]
    fn rejects_wrong_pixel_buffer_size() {
        assert!(DecodedImage::from_rgba8(2, 2, vec![0; 15]).is_err());
    }
}
