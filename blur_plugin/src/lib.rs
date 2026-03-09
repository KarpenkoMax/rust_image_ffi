//! Blur image plugin (`cdylib`) for the `image_processor` host.
//!
//! The plugin exports a single C ABI entrypoint: [`process_image`].
//! The host passes:
//! - image dimensions (`width`, `height`);
//! - mutable RGBA8 pixel buffer (`width * height * 4` bytes);
//! - JSON params as a NUL-terminated UTF-8 C string.
//!
//! Expected params JSON:
//! `{"radius": <u32>, "iterations": <u32>}`
//!
//! Processing is done in-place. On invalid input, the plugin logs an error
//! to stderr and returns without panicking.
#![warn(missing_docs)]

use std::ffi::CStr;

#[derive(Debug, serde::Deserialize)]
struct BlurParams {
    radius: u32,
    iterations: u32,
}

/// Applies blur transformation to an RGBA image buffer in-place.
///
/// # Parameters
/// - `width`: image width in pixels.
/// - `height`: image height in pixels.
/// - `rgba_data`: pointer to a mutable buffer of `width * height * 4` bytes
///   in RGBA8 layout.
/// - `params`: pointer to a NUL-terminated UTF-8 JSON string with shape:
///   `{"radius": u32, "iterations": u32}`.
///
/// # Behavior
/// The function validates pointers/params and returns early on errors, logging
/// details to stderr. On success, it mutates `rgba_data` in-place.
///
/// # Safety contract (caller side)
/// - `rgba_data` must be non-null and valid for writes for the full buffer size.
/// - `params` must be non-null and point to a valid NUL-terminated C string.
/// - The memory behind pointers must stay valid for the duration of the call.
#[unsafe(no_mangle)]
pub extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const std::os::raw::c_char,
) {
    if rgba_data.is_null() {
        eprintln!("[blur_plugin] rgba_data is null");
        return;
    }
    if params.is_null() {
        eprintln!("[blur_plugin] params is null");
        return;
    }

    // SAFETY:
    // `params` is checked for null above and must be a valid NUL-terminated
    // C string according to the FFI contract.
    let Ok(params_str) = (unsafe { CStr::from_ptr(params) }).to_str() else {
        eprintln!("[blur_plugin] params is not valid UTF-8");
        return;
    };

    let cfg: BlurParams = match serde_json::from_str(params_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[blur_plugin] invalid JSON params: {e}");
            return;
        }
    };

    if cfg.iterations == 0 {
        eprintln!("[blur_plugin] iterations must be >= 1");
        return;
    }

    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        eprintln!("[blur_plugin] overflow while computing pixel count");
        return;
    };
    let Some(len) = pixel_count.checked_mul(4) else {
        eprintln!("[blur_plugin] overflow while computing RGBA buffer length");
        return;
    };

    if len == 0 {
        eprintln!("[blur_plugin] empty image buffer (width={width}, height={height})");
        return;
    }

    // SAFETY:
    // pointers checked for null
    // FFI contract guarantees that rgba_data points to a writable buffer of len bytes
    let buf = unsafe { std::slice::from_raw_parts_mut(rgba_data, len) };

    apply_blur(
        buf,
        width as usize,
        height as usize,
        cfg.radius as usize,
        cfg.iterations,
    );
}

fn apply_blur(buf: &mut [u8], width: usize, height: usize, radius: usize, iterations: u32) {
    if radius == 0 {
        return;
    }

    for _ in 0..iterations {
        blur_once(buf, width, height, radius);
    }
}

fn blur_once(buf: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 {
        return;
    }

    let src = buf.to_vec();
    let r_sqr = (radius * radius) as i64;

    for y in 0..height {
        let y0 = y.saturating_sub(radius);
        let y1 = (y + radius).min(height - 1);

        for x in 0..width {
            let x0 = x.saturating_sub(radius);
            let x1 = (x + radius).min(width - 1);

            let mut sum_r: f64 = 0.0;
            let mut sum_g: f64 = 0.0;
            let mut sum_b: f64 = 0.0;
            let mut sum_w: f64 = 0.0;

            for ny in y0..=y1 {
                for nx in x0..=x1 {
                    let dx = nx as i64 - x as i64;
                    let dy = ny as i64 - y as i64;
                    let d_sqr = dx * dx + dy * dy;
                    if d_sqr > r_sqr {
                        continue; // outside circle
                    }

                    // Linear falloff
                    let dist = (d_sqr as f64).sqrt();
                    let weight = 1.0 - dist / (radius as f64 + 1.0);
                    if weight <= 0.0 {
                        continue;
                    }

                    let i = (ny * width + nx) * 4;
                    sum_r += src[i] as f64 * weight;
                    sum_g += src[i + 1] as f64 * weight;
                    sum_b += src[i + 2] as f64 * weight;
                    sum_w += weight;
                }
            }

            let out = (y * width + x) * 4;
            if sum_w > 0.0 {
                buf[out] = (sum_r / sum_w).round().clamp(0.0, 255.0) as u8;
                buf[out + 1] = (sum_g / sum_w).round().clamp(0.0, 255.0) as u8;
                buf[out + 2] = (sum_b / sum_w).round().clamp(0.0, 255.0) as u8;
            } else {
                buf[out] = src[out];
                buf[out + 1] = src[out + 1];
                buf[out + 2] = src[out + 2];
            }

            // keep alpha unchanged
            buf[out + 3] = src[out + 3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BlurParams, apply_blur, blur_once};

    #[test]
    fn parses_valid_params() {
        let json = r#"{"radius":2,"iterations":3}"#;
        let params: BlurParams = serde_json::from_str(json).expect("valid params");
        assert_eq!(params.radius, 2);
        assert_eq!(params.iterations, 3);
    }

    #[test]
    fn apply_blur_with_zero_radius_is_noop() {
        let mut buf = vec![10, 20, 30, 255, 40, 50, 60, 255];
        let original = buf.clone();

        apply_blur(&mut buf, 2, 1, 0, 1);

        assert_eq!(buf, original);
    }

    #[test]
    fn blur_once_radius_one_changes_rgb() {
        let mut buf = vec![
            0, 0, 0, 255, // x=0
            255, 0, 0, 255, // x=1
            0, 0, 0, 255, // x=2
        ];

        blur_once(&mut buf, 3, 1, 1);

        // Edges should become non-zero due to neighbor contribution.
        assert!(buf[0] > 0);
        assert!(buf[8] > 0);
        // Center should no longer be the original full intensity.
        assert!(buf[4] < 255);
    }

    #[test]
    fn blur_once_preserves_alpha_channel() {
        let mut buf = vec![10, 20, 30, 1, 40, 50, 60, 77, 70, 80, 90, 255];
        let alpha_before: Vec<u8> = buf.chunks_exact(4).map(|p| p[3]).collect();

        blur_once(&mut buf, 3, 1, 1);

        let alpha_after: Vec<u8> = buf.chunks_exact(4).map(|p| p[3]).collect();
        assert_eq!(alpha_before, alpha_after);
    }
}
