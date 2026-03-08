use std::path::Path;

use crate::error::AppError;

type ProcessImageFn = unsafe extern "C" fn(u32, u32, *mut u8, *const std::os::raw::c_char);

pub(crate) struct Plugin {
    lib: libloading::Library,
}

impl Plugin {
    pub(crate) fn load(path: &Path) -> Result<Self, AppError> {
        // SAFETY:
        // Loading a dynamic library is inherently unsafe because constructors in the
        // library may run and symbol types are trusted by the caller.
        // Here we only open a path provided by validated CLI input and keep `Library`
        // owned by `Plugin`, so symbol lifetimes cannot outlive the loaded library.
        let lib = unsafe { libloading::Library::new(path)? };
        Ok(Self { lib })
    }

    pub(crate) fn process(
        &self,
        width: u32,
        height: u32,
        rgba: &mut [u8],
        params: &str,
    ) -> Result<(), AppError> {
        validate_rgba(width, height, rgba)?;

        // SAFETY:
        // We request symbol `process_image` with the exact ABI/signature agreed by the
        // plugin contract. `self.lib` is alive for the full duration of symbol usage.
        let process_image: libloading::Symbol<'_, ProcessImageFn> =
            unsafe { self.lib.get(b"process_image\0") }?;
        let c_params = std::ffi::CString::new(params).map_err(|_| AppError::InvalidParams)?;

        // SAFETY:
        // - `rgba.as_mut_ptr()` points to a valid mutable buffer.
        // - `validate_rgba` guarantees buffer len == width * height * 4.
        // - `c_params.as_ptr()` is NUL-terminated and valid for this call.
        // - Both pointers remain valid until `process_image` returns.
        unsafe { process_image(width, height, rgba.as_mut_ptr(), c_params.as_ptr()) };
        Ok(())
    }
}

fn validate_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<(), AppError> {
    if rgba.len() != (width as usize * height as usize * 4) {
        return Err(AppError::InvalidImageBuffer);
    }
    Ok(())
}
