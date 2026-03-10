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

        if let Some(parent) = self.output.parent()
            && !parent.exists()
        {
            return Err(AppError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("output directory not found: {}", parent.display()),
            )));
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
    use crate::error::AppError;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "image_processor_cli_test_{}_{}",
            std::process::id(),
            nanos
        ))
    }

    fn write_file(path: &Path, content: &str) {
        fs::write(path, content).expect("failed to write test file");
    }

    fn base_args(root: &Path) -> Args {
        let input = root.join("in.png");
        let params = root.join("params.json");
        let output = root.join("out.png");
        let plugin_dir = root.join("plugins");

        write_file(&input, "not a real png for validation test");
        write_file(&params, r#"{"horizontal": true}"#);
        fs::create_dir_all(&plugin_dir).expect("failed to create plugin dir");

        Args {
            plugin: "mirror_plugin".to_string(),
            input,
            output,
            plugin_path: plugin_dir,
            params,
        }
    }

    #[test]
    fn png_detection_works() {
        assert!(Args::output_is_png_path(&PathBuf::from("out.png")));
        assert!(Args::output_is_png_path(&PathBuf::from("out.PNG")));
        assert!(!Args::output_is_png_path(&PathBuf::from("out.jpg")));
        assert!(!Args::output_is_png_path(&PathBuf::from("out")));
    }

    #[test]
    fn validate_passes_for_existing_files_and_dirs() {
        let root = unique_test_dir();
        fs::create_dir_all(&root).expect("failed to create test dir");

        let args = base_args(&root);
        assert!(args.validate().is_ok());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validate_fails_when_input_missing() {
        let root = unique_test_dir();
        fs::create_dir_all(&root).expect("failed to create test dir");

        let mut args = base_args(&root);
        args.input = root.join("missing.png");
        let result = args.validate();

        assert!(matches!(result, Err(AppError::Io(_))));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validate_fails_when_params_missing() {
        let root = unique_test_dir();
        fs::create_dir_all(&root).expect("failed to create test dir");

        let mut args = base_args(&root);
        args.params = root.join("missing.json");
        let result = args.validate();

        assert!(matches!(result, Err(AppError::Io(_))));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validate_fails_when_plugin_path_is_not_directory() {
        let root = unique_test_dir();
        fs::create_dir_all(&root).expect("failed to create test dir");

        let mut args = base_args(&root);
        let fake_plugin_path = root.join("not_a_dir.txt");
        write_file(&fake_plugin_path, "x");
        args.plugin_path = fake_plugin_path;

        let result = args.validate();
        assert!(matches!(result, Err(AppError::InvalidPluginPath)));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validate_fails_for_non_png_output() {
        let root = unique_test_dir();
        fs::create_dir_all(&root).expect("failed to create test dir");

        let mut args = base_args(&root);
        args.output = root.join("out.jpg");

        let result = args.validate();
        assert!(matches!(result, Err(AppError::InvalidParams)));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_lib_path_uses_platform_extension() {
        let args = Args {
            plugin: "mirror_plugin".to_string(),
            input: PathBuf::from("in.png"),
            output: PathBuf::from("out.png"),
            plugin_path: PathBuf::from("/tmp/plugins"),
            params: PathBuf::from("params.json"),
        };

        let path = args.plugin_lib_path();
        #[cfg(target_os = "linux")]
        assert!(path.ends_with("libmirror_plugin.so"));
        #[cfg(target_os = "macos")]
        assert!(path.ends_with("libmirror_plugin.dylib"));
        #[cfg(target_os = "windows")]
        assert!(path.ends_with("mirror_plugin.dll"));
    }
}
