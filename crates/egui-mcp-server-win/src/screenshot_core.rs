use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

const MAX_NORMALIZED_DISTANCE: f64 = 1.0;
const MIN_NORMALIZED_DISTANCE: f64 = 0.0;

#[derive(Debug, Clone, PartialEq)]
pub struct CompareResult {
    pub similarity: f64,
    pub is_match: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffResult {
    pub diff_image: RgbaImage,
    pub changed_pixels: u64,
    pub change_ratio: f64,
}

pub fn encode_png_base64(image: &RgbaImage) -> Result<String, String> {
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image.clone())
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|err| format!("Failed to encode PNG: {}", err))?;

    Ok(general_purpose::STANDARD.encode(cursor.into_inner()))
}

pub fn decode_png_base64(image_base64: &str) -> Result<RgbaImage, String> {
    let encoded = image_base64.trim();
    if encoded.is_empty() {
        return Err("Image payload cannot be empty".to_string());
    }

    let image_bytes = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|err| format!("Invalid base64 image payload: {}", err))?;

    image::load_from_memory_with_format(&image_bytes, ImageFormat::Png)
        .map_err(|err| format!("Failed to decode PNG image payload: {}", err))
        .map(|image| image.to_rgba8())
}

pub fn parse_highlight_color(value: Option<&str>) -> Result<[u8; 4], String> {
    let raw = value.unwrap_or("#FF0055").trim();
    let hex = raw.strip_prefix('#').unwrap_or(raw);

    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| format!("Invalid highlight_color '{}': expected hex RGB", raw))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| format!("Invalid highlight_color '{}': expected hex RGB", raw))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| format!("Invalid highlight_color '{}': expected hex RGB", raw))?;
            Ok([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| format!("Invalid highlight_color '{}': expected hex RGBA", raw))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| format!("Invalid highlight_color '{}': expected hex RGBA", raw))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| format!("Invalid highlight_color '{}': expected hex RGBA", raw))?;
            let a = u8::from_str_radix(&hex[6..8], 16)
                .map_err(|_| format!("Invalid highlight_color '{}': expected hex RGBA", raw))?;
            Ok([r, g, b, a])
        }
        _ => Err(format!(
            "Invalid highlight_color '{}': expected '#RRGGBB' or '#RRGGBBAA'",
            raw
        )),
    }
}

pub fn compare_images(
    left: &RgbaImage,
    right: &RgbaImage,
    threshold: f64,
) -> Result<CompareResult, String> {
    ensure_same_dimensions(left, right)?;
    validate_unit_interval(threshold, "threshold")?;

    let similarity = calculate_similarity(left, right);

    Ok(CompareResult {
        similarity,
        is_match: similarity >= threshold,
        width: left.width(),
        height: left.height(),
    })
}

pub fn diff_images(
    left: &RgbaImage,
    right: &RgbaImage,
    sensitivity: f64,
    highlight_color: [u8; 4],
) -> Result<DiffResult, String> {
    ensure_same_dimensions(left, right)?;
    validate_unit_interval(sensitivity, "sensitivity")?;

    let mut changed_pixels: u64 = 0;
    let mut diff_image = RgbaImage::new(left.width(), left.height());

    for (x, y, left_pixel) in left.enumerate_pixels() {
        let right_pixel = right.get_pixel(x, y);
        let distance = pixel_distance(*left_pixel, *right_pixel);

        if distance > sensitivity {
            changed_pixels += 1;
            diff_image.put_pixel(x, y, Rgba(highlight_color));
        } else {
            // Keep unchanged regions visible but muted.
            let gray =
                ((left_pixel[0] as u16 + left_pixel[1] as u16 + left_pixel[2] as u16) / 3) as u8;
            diff_image.put_pixel(x, y, Rgba([gray, gray, gray, 255]));
        }
    }

    let total_pixels = (left.width() as u64) * (left.height() as u64);
    let change_ratio = if total_pixels == 0 {
        0.0
    } else {
        changed_pixels as f64 / total_pixels as f64
    };

    Ok(DiffResult {
        diff_image,
        changed_pixels,
        change_ratio,
    })
}

fn ensure_same_dimensions(left: &RgbaImage, right: &RgbaImage) -> Result<(), String> {
    if left.width() != right.width() || left.height() != right.height() {
        return Err(format!(
            "Image dimensions must match: left={}x{}, right={}x{}",
            left.width(),
            left.height(),
            right.width(),
            right.height()
        ));
    }

    Ok(())
}

fn validate_unit_interval(value: f64, label: &str) -> Result<(), String> {
    if !(MIN_NORMALIZED_DISTANCE..=MAX_NORMALIZED_DISTANCE).contains(&value) {
        return Err(format!("{} must be between 0.0 and 1.0", label));
    }

    Ok(())
}

fn calculate_similarity(left: &RgbaImage, right: &RgbaImage) -> f64 {
    let mut total_distance = 0.0;
    let total_pixels = (left.width() as usize) * (left.height() as usize);

    if total_pixels == 0 {
        return 1.0;
    }

    for (left_pixel, right_pixel) in left.pixels().zip(right.pixels()) {
        total_distance += pixel_distance(*left_pixel, *right_pixel);
    }

    let normalized_distance = total_distance / total_pixels as f64;
    (1.0 - normalized_distance).clamp(0.0, 1.0)
}

fn pixel_distance(left: Rgba<u8>, right: Rgba<u8>) -> f64 {
    let r = (left[0] as i16 - right[0] as i16).unsigned_abs() as f64 / 255.0;
    let g = (left[1] as i16 - right[1] as i16).unsigned_abs() as f64 / 255.0;
    let b = (left[2] as i16 - right[2] as i16).unsigned_abs() as f64 / 255.0;
    let a = (left[3] as i16 - right[3] as i16).unsigned_abs() as f64 / 255.0;

    (r + g + b + a) / 4.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn solid_image(width: u32, height: u32, color: [u8; 4]) -> RgbaImage {
        RgbaImage::from_fn(width, height, |_x, _y| Rgba(color))
    }

    #[test]
    fn png_base64_round_trip_preserves_pixels() {
        let input = solid_image(4, 3, [12, 34, 56, 255]);
        let encoded = encode_png_base64(&input).expect("encode");
        let decoded = decode_png_base64(&encoded).expect("decode");
        assert_eq!(decoded, input);
    }

    #[test]
    fn compare_reports_full_similarity_for_identical_images() {
        let image = solid_image(2, 2, [255, 0, 0, 255]);
        let result = compare_images(&image, &image, 0.99).expect("compare");
        assert_eq!(result.similarity, 1.0);
        assert!(result.is_match);
    }

    #[test]
    fn compare_detects_non_matching_images() {
        let left = solid_image(2, 2, [0, 0, 0, 255]);
        let right = solid_image(2, 2, [255, 255, 255, 255]);
        let result = compare_images(&left, &right, 0.95).expect("compare");
        assert!(!result.is_match);
        assert!(result.similarity < 0.95);
    }

    #[test]
    fn diff_marks_changed_pixels_and_ratio() {
        let mut left = solid_image(2, 1, [0, 0, 0, 255]);
        let mut right = solid_image(2, 1, [0, 0, 0, 255]);
        right.put_pixel(1, 0, Rgba([255, 0, 0, 255]));

        let diff = diff_images(&left, &right, 0.01, [0, 255, 0, 255]).expect("diff");
        assert_eq!(diff.changed_pixels, 1);
        assert_eq!(diff.change_ratio, 0.5);
        assert_eq!(diff.diff_image.get_pixel(1, 0).0, [0, 255, 0, 255]);

        left.put_pixel(1, 0, Rgba([255, 0, 0, 255]));
        let diff_same = diff_images(&left, &right, 0.01, [0, 255, 0, 255]).expect("diff");
        assert_eq!(diff_same.changed_pixels, 0);
        assert_eq!(diff_same.change_ratio, 0.0);
    }
}
