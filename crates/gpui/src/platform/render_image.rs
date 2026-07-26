use anyhow::Result;

#[allow(dead_code)]
pub fn rgba_image_from_bgra_rows(
    width: u32,
    height: u32,
    row_pitch: usize,
    bytes: &[u8],
) -> Result<image::RgbaImage> {
    rgba_image_from_gpu_rows(width, height, row_pitch, bytes, true)
}

#[allow(dead_code)]
pub fn rgba_image_from_rgba_rows(
    width: u32,
    height: u32,
    row_pitch: usize,
    bytes: &[u8],
) -> Result<image::RgbaImage> {
    rgba_image_from_gpu_rows(width, height, row_pitch, bytes, false)
}

fn rgba_image_from_gpu_rows(
    width: u32,
    height: u32,
    row_pitch: usize,
    bytes: &[u8],
    swap_red_blue: bool,
) -> Result<image::RgbaImage> {
    let row_bytes = width
        .checked_mul(4)
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| anyhow::anyhow!("rendered image row size overflow"))?;
    if row_pitch < row_bytes {
        anyhow::bail!("readback row pitch {row_pitch} is smaller than {row_bytes}");
    }

    let required_len = row_pitch
        .checked_mul(height as usize)
        .ok_or_else(|| anyhow::anyhow!("readback buffer size overflow"))?;
    if bytes.len() < required_len {
        anyhow::bail!(
            "readback buffer is too short: expected at least {required_len} bytes, got {}",
            bytes.len()
        );
    }

    let pixel_len = row_bytes
        .checked_mul(height as usize)
        .ok_or_else(|| anyhow::anyhow!("rendered image buffer size overflow"))?;
    let mut pixels = Vec::with_capacity(pixel_len);
    for row in bytes.chunks_exact(row_pitch).take(height as usize) {
        pixels.extend_from_slice(&row[..row_bytes]);
    }
    if swap_red_blue {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }

    image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| anyhow::anyhow!("failed to create image from GPU readback buffer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_bgra_pixels_to_rgba() {
        let image = rgba_image_from_bgra_rows(2, 1, 8, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();

        assert_eq!(image.get_pixel(0, 0).0, [3, 2, 1, 4]);
        assert_eq!(image.get_pixel(1, 0).0, [7, 6, 5, 8]);
    }

    #[test]
    fn preserves_rgba_channel_order() {
        let image = rgba_image_from_rgba_rows(1, 1, 4, &[1, 2, 3, 4]).unwrap();

        assert_eq!(image.get_pixel(0, 0).0, [1, 2, 3, 4]);
    }

    #[test]
    fn removes_gpu_row_padding() {
        let image = rgba_image_from_bgra_rows(
            1,
            2,
            8,
            &[
                10, 20, 30, 40, 0, 0, 0, 0, // first padded row
                50, 60, 70, 80, 0, 0, 0, 0, // second padded row
            ],
        )
        .unwrap();

        assert_eq!(image.get_pixel(0, 0).0, [30, 20, 10, 40]);
        assert_eq!(image.get_pixel(0, 1).0, [70, 60, 50, 80]);
    }

    #[test]
    fn rejects_short_readback_buffers() {
        let error = rgba_image_from_bgra_rows(2, 2, 8, &[0; 15]).unwrap_err();

        assert!(error.to_string().contains("readback buffer"));
    }
}
