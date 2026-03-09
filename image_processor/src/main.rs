mod cli;
mod error;
mod plugin_loader;

use crate::error::AppError;
use crate::plugin_loader::Plugin;

fn main() -> Result<(), AppError> {
    run()
}

fn run() -> Result<(), AppError> {
    let args = cli::Args::parse_args();
    args.validate()?;

    let img_buf = image::open(&args.input)?.to_rgba8();

    let width = img_buf.width();
    let height = img_buf.height();
    let params = std::fs::read_to_string(&args.params)?;

    let plugin_path = args.plugin_lib_path();
    let plugin = Plugin::load(&plugin_path)?;

    let mut rgba = img_buf.into_raw();
    plugin.process(width, height, &mut rgba, &params)?;

    let out =
        image::RgbaImage::from_raw(width, height, rgba).ok_or(AppError::InvalidImageBuffer)?;

    out.save(&args.output)?;

    Ok(())
}
