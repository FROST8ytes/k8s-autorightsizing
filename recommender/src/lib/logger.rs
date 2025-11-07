use directories::ProjectDirs;
use log::LevelFilter;
use std::fs;
use std::io::Write;

use crate::Result;

/// Initialize the logger with file and console output
///
/// # Arguments
///
/// * `verbose` - Enable debug level logging
/// * `quiet` - Suppress console output (logs still written to file)
///
/// # Platform-specific log locations
///
/// * **macOS**: `~/Library/Application Support/com.frost8ytes.k8s-recommender/recommender.log`
/// * **Linux**: `~/.local/share/k8s-recommender/recommender.log`
/// * **Windows**: `C:\Users\<User>\AppData\Local\frost8ytes\k8s-recommender\data\recommender.log`
///
pub fn init_logger(verbose: bool, quiet: bool) -> Result<()> {
    let log_level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    // Get platform-specific log directory
    let log_path = if let Some(proj_dirs) =
        ProjectDirs::from("com", "frost8ytes", "k8s-recommender")
    {
        let log_dir = proj_dirs.data_local_dir();
        fs::create_dir_all(log_dir).map_err(|e| {
            crate::ConfigError::InvalidValue(format!("Failed to create log directory: {}", e))
        })?;
        log_dir.join("recommender.log")
    } else {
        // Fallback to current directory if ProjectDirs fails
        std::env::current_dir()
            .map_err(|e| {
                crate::ConfigError::InvalidValue(format!("Failed to get current directory: {}", e))
            })?
            .join("recommender.log")
    };

    // Open log file for writing
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(|e| crate::ConfigError::InvalidValue(format!("Failed to open log file: {}", e)))?;

    // Build logger
    let mut builder = env_logger::Builder::new();
    builder.filter_level(log_level).format_timestamp_secs();

    if quiet {
        // Only write to file when quiet
        builder.target(env_logger::Target::Pipe(Box::new(log_file)));
    } else {
        // Write to both stdout and file
        struct MultiWriter {
            stdout: std::io::Stdout,
            file: fs::File,
        }

        impl Write for MultiWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.stdout.write_all(buf)?;
                self.file.write_all(buf)?;
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                self.stdout.flush()?;
                self.file.flush()?;
                Ok(())
            }
        }

        let multi_writer = MultiWriter {
            stdout: std::io::stdout(),
            file: log_file,
        };
        builder.target(env_logger::Target::Pipe(Box::new(multi_writer)));
    }

    builder.init();

    if !quiet {
        log::info!("Logging to: {}", log_path.display());
    }

    Ok(())
}
