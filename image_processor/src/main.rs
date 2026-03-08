mod cli;
mod error;
mod plugin_loader;
use image;

use crate::error::AppError;

fn main() -> Result<(), AppError> {
    run()?;
    Ok(())
}

fn run() -> Result<(), AppError> {
    let args = cli::Args::parse_args();
    args.validate()?;

    let img_buf = image::open(&args.input)?.to_rgba8();

    let width = img_buf.width();
    let height = img_buf.height();
    let params = std::fs::read_to_string(&args.params)?;

    let out = image::RgbaImage::from_raw(width, height, img_buf.into_raw())
        .ok_or(AppError::InvalidImageBuffer)?;

    out.save(&args.output)?;

    Ok(())
}
