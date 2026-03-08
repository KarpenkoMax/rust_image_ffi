use crate::error::AppError;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "image-processor", version, about = "Image processor")]
pub(crate) struct Args {
    #[arg(long)]
    pub(crate) plugin: String,

    #[arg(long)]
    pub(crate) input: PathBuf,

    #[arg(long)]
    pub(crate) output: PathBuf,

    #[arg(long, default_value = "target/debug")]
    pub(crate) plugin_path: PathBuf,

    #[arg(long)]
    pub(crate) params: PathBuf,
}

impl Args {
    pub(crate) fn parse_args() -> Self {
        Self::parse()
    }

    pub(crate) fn validate(&self) -> Result<(), AppError> {
        if self.plugin.trim().is_empty() {
            return Err(AppError::InvalidParams);
        }

        if !self.input.is_file() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("input file not found: {}", self.input.display()),
            )));
        }

        if !self.params.is_file() {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("params file not found: {}", self.params.display()),
            )));
        }

        if !self.plugin_path.is_dir() {
            return Err(AppError::InvalidPluginPath);
        }

        if let Some(parent) = self.output.parent() {
            if !parent.exists() {
                return Err(AppError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("output directory not found: {}", parent.display()),
                )));
            }
        }

        self.output_is_png()?;
        Ok(())
    }

    pub(crate) fn output_is_png(&self) -> Result<(), AppError> {
        let is_png = self
            .output
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            .unwrap_or(false);

        if !is_png {
            return Err(AppError::InvalidParams);
        }

        Ok(())
    }

    pub(crate) fn plugin_lib_path(&self) -> PathBuf {
        #[cfg(target_os = "windows")]
        let file_name = format!("{}.dll", self.plugin);
        #[cfg(target_os = "linux")]
        let file_name = format!("lib{}.so", self.plugin);
        #[cfg(target_os = "macos")]
        let file_name = format!("lib{}.dylib", self.plugin);

        self.plugin_path.join(file_name)
    }

    #[cfg(test)]
    pub(crate) fn output_is_png_path(path: &PathBuf) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use std::path::PathBuf;

    #[test]
    fn png_detection_works() {
        assert!(Args::output_is_png_path(&PathBuf::from("out.png")));
        assert!(Args::output_is_png_path(&PathBuf::from("out.PNG")));
        assert!(!Args::output_is_png_path(&PathBuf::from("out.jpg")));
        assert!(!Args::output_is_png_path(&PathBuf::from("out")));
    }
}
