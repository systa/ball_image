use image::{ImageBuffer, Rgba, RgbaImage};

/// Resize an RGBA image using bilinear interpolation.
///
/// This is a Rust equivalent of the original Python `bl_resize` function.
/// Source: https://meghal-darji.medium.com/implementing-bilinear-interpolation-for-image-resizing-357cbb2c2722
pub fn bl_resize(original: &RgbaImage, new_h: u32, new_w: u32) -> RgbaImage {
    if new_h == 0 || new_w == 0 {
        return ImageBuffer::new(new_w, new_h);
    }

    let (old_w, old_h) = original.dimensions();
    if old_w == 0 || old_h == 0 {
        return ImageBuffer::new(new_w, new_h);
    }

    let w_scale = old_w as f32 / new_w as f32;
    let h_scale = old_h as f32 / new_h as f32;
    let mut resized = ImageBuffer::new(new_w, new_h);

    for i in 0..new_h {
        for j in 0..new_w {
            let x = i as f32 * h_scale;
            let y = j as f32 * w_scale;

            let x_floor = x.floor() as u32;
            let x_ceil = x.ceil().min((old_h - 1) as f32) as u32;
            let y_floor = y.floor() as u32;
            let y_ceil = y.ceil().min((old_w - 1) as f32) as u32;

            let v1 = original.get_pixel(y_floor, x_floor);
            let v2 = original.get_pixel(y_floor, x_ceil);
            let v3 = original.get_pixel(y_ceil, x_floor);
            let v4 = original.get_pixel(y_ceil, x_ceil);

            let x_weight = x - x_floor as f32;
            let y_weight = y - y_floor as f32;

            let mut pixel = [0u8; 4];
            for channel in 0..4 {
                let c1 = v1.0[channel] as f32;
                let c2 = v2.0[channel] as f32;
                let c3 = v3.0[channel] as f32;
                let c4 = v4.0[channel] as f32;

                let q1 = c1 * (1.0 - x_weight) + c2 * x_weight;
                let q2 = c3 * (1.0 - x_weight) + c4 * x_weight;
                let q = q1 * (1.0 - y_weight) + q2 * y_weight;

                pixel[channel] = q.clamp(0.0, 255.0) as u8;
            }

            resized.put_pixel(j, i, Rgba(pixel));
        }
    }

    resized
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn resize_preserves_channel_count() {
        let original = ImageBuffer::from_fn(2, 2, |x, y| {
            Rgba([x as u8 * 100, y as u8 * 100, 50, 255])
        });

        let resized = bl_resize(&original, 4, 4);
        assert_eq!(resized.dimensions(), (4, 4));
        assert_eq!(resized.get_pixel(0, 0).0[3], 255);
    }
}
