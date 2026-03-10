use thiserror::Error;

///  Image processor errors
#[derive(Debug, Error)]
pub enum AppError {
    #[error("invalid plugin path")]
    InvalidPluginPath,
    #[error("invalid parameters")]
    InvalidParams,
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("image error")]
    Image(#[from] image::ImageError),
    #[error("invalid image buffer")]
    InvalidImageBuffer,
    #[error("plugin error")]
    Plugin(#[from] libloading::Error),
}
