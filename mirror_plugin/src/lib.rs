//! Mirror image plugin (`cdylib`) for the `image_processor` host.
//!
//! The plugin exports a single C ABI entrypoint: [`process_image`].
//! The host passes:
//! - image dimensions (`width`, `height`);
//! - mutable RGBA8 pixel buffer (`width * height * 4` bytes);
//! - JSON params as a NUL-terminated UTF-8 C string.
//!
//! Expected params JSON:
//! `{"horizontal": <bool>, "vertical": <bool>}`
//!
//! Processing is done in-place. On invalid input, the plugin logs an error
//! to stderr and returns without panicking.
#![warn(missing_docs)]

use std::ffi::CStr;
use std::sync::Once;

static INIT_LOGGER: Once = Once::new();

fn init_logger() {
    INIT_LOGGER.call_once(|| {
        let _ = env_logger::try_init();
    });
}

#[derive(serde::Deserialize)]
struct MirrorParams {
    horizontal: bool,
    vertical: bool,
}

/// Applies mirror transformation to an RGBA image buffer in-place.
///
/// # Parameters
/// - `width`: image width in pixels.
/// - `height`: image height in pixels.
/// - `rgba_data`: pointer to a mutable buffer of `width * height * 4` bytes
///   in RGBA8 layout.
/// - `params`: pointer to a NUL-terminated UTF-8 JSON string with shape:
///   `{"horizontal": bool, "vertical": bool}`.
///
/// # Behavior
/// The function validates pointers/params and returns early on errors, logging
/// details to stderr. On success, it mutates `rgba_data` in-place.
///
/// # Safety
/// Caller must guarantee:
/// - `rgba_data` is non-null and valid for writes for the full buffer size.
/// - `params` is non-null and points to a valid NUL-terminated C string.
/// - The memory behind pointers stays valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const std::os::raw::c_char,
) {
    init_logger();

    if rgba_data.is_null() {
        log::error!("[mirror_plugin] rgba_data is null");
        return;
    }
    if params.is_null() {
        log::error!("[mirror_plugin] params is null");
        return;
    }
    // SAFETY:
    // `params` is checked for null above and is expected to point to a valid
    // NUL-terminated C string according to the FFI contract.
    let Ok(params_str) = (unsafe { CStr::from_ptr(params) }).to_str() else {
        log::error!("[mirror_plugin] params is not valid UTF-8");
        return;
    };

    let cfg: MirrorParams = match serde_json::from_str(params_str) {
        Ok(v) => v,
        Err(e) => {
            log::error!("[mirror_plugin] invalid JSON params: {e}");
            return;
        }
    };

    let Some(pixel_count) = (width as usize).checked_mul(height as usize) else {
        log::error!("[mirror_plugin] overflow while computing pixel count");
        return;
    };

    let Some(len) = pixel_count.checked_mul(4) else {
        log::error!("[mirror_plugin] overflow while computing RGBA buffer length");
        return;
    };

    if len == 0 {
        log::error!("[mirror_plugin] empty image buffer (width={width}, height={height})");
        return;
    }

    // SAFETY:
    // pointers checked for null
    // FFI contract guarantees that rgba_data points to a writable buffer of len bytes
    let buf = unsafe { std::slice::from_raw_parts_mut(rgba_data, len) };

    if cfg.horizontal {
        mirror_horizontal(buf, width as usize, height as usize);
    }
    if cfg.vertical {
        mirror_vertical(buf, width as usize, height as usize);
    }
}

fn mirror_horizontal(buf: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..(width / 2) {
            let left = (y * width + x) * 4;
            let right = (y * width + (width - x - 1)) * 4;
            for c in 0..4 {
                buf.swap(left + c, right + c);
            }
        }
    }
}

fn mirror_vertical(buf: &mut [u8], width: usize, height: usize) {
    for x in 0..width {
        for y in 0..(height / 2) {
            let top = (y * width + x) * 4;
            let bot = ((height - y - 1) * width + x) * 4;
            for c in 0..4 {
                buf.swap(top + c, bot + c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mirror_horizontal, mirror_vertical};

    #[test]
    fn mirror_horizontal_2x1() {
        let mut buf = vec![
            1, 2, 3, 4, // px0
            5, 6, 7, 8, // px1
        ];

        mirror_horizontal(&mut buf, 2, 1);

        assert_eq!(
            buf,
            vec![
                5, 6, 7, 8, // px1
                1, 2, 3, 4, // px0
            ]
        );
    }

    #[test]
    fn mirror_vertical_1x2() {
        let mut buf = vec![
            1, 2, 3, 4, // top
            5, 6, 7, 8, // bottom
        ];

        mirror_vertical(&mut buf, 1, 2);

        assert_eq!(
            buf,
            vec![
                5, 6, 7, 8, // bottom
                1, 2, 3, 4, // top
            ]
        );
    }

    #[test]
    fn mirror_both_2x2() {
        let mut buf = vec![
            1, 1, 1, 255, // (0,0)
            2, 2, 2, 255, // (1,0)
            3, 3, 3, 255, // (0,1)
            4, 4, 4, 255, // (1,1)
        ];

        mirror_horizontal(&mut buf, 2, 2);
        mirror_vertical(&mut buf, 2, 2);

        assert_eq!(
            buf,
            vec![
                4, 4, 4, 255, // (1,1)
                3, 3, 3, 255, // (0,1)
                2, 2, 2, 255, // (1,0)
                1, 1, 1, 255, // (0,0)
            ]
        );
    }
}
