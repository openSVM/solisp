//! # Lean 4 Bridge
//!
//! This module provides communication with the Lean 4 theorem prover via subprocess.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// Bridge to Lean 4 theorem prover
pub struct LeanBridge {
    /// Path to lean executable
    lean_path: PathBuf,
    /// Path to lake executable
    lake_path: PathBuf,
    /// Path to OVSM Lean library
    ovsm_lib_path: PathBuf,
    /// Verification timeout
    timeout: Duration,
    /// Whether Lean 4 is available
    available: bool,
}

impl LeanBridge {
    /// Create a new Lean bridge
    pub fn new(
        lean_path: Option<PathBuf>,
        ovsm_lib: Option<PathBuf>,
        timeout_secs: u64,
    ) -> Result<Self> {
        // Find lean executable
        let lean_path = lean_path.unwrap_or_else(|| PathBuf::from("lean"));
        let lake_path = PathBuf::from("lake");

        // Find OVSM Lean library
        let ovsm_lib_path = ovsm_lib.unwrap_or_else(|| {
            // Default: look relative to crate root
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("lean");
            path
        });

        // Check if Lean is available
        let available = Self::check_lean_available(&lean_path);

        Ok(Self {
            lean_path,
            lake_path,
            ovsm_lib_path,
            timeout: Duration::from_secs(timeout_secs),
            available,
        })
    }

    /// Check if Lean 4 is installed and available
    fn check_lean_available(lean_path: &PathBuf) -> bool {
        Command::new(lean_path)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Check if Lean 4 is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Get Lean 4 version
    pub fn version(&self) -> Result<String> {
        if !self.available {
            return Err(Error::compiler("Lean 4 is not available".to_string()));
        }

        let output = Command::new(&self.lean_path)
            .arg("--version")
            .output()
            .map_err(|e| Error::compiler(format!("Failed to run lean: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(Error::compiler("Failed to get Lean version".to_string()))
        }
    }

    /// Check a Lean file for errors
    pub fn check_file(&self, file: &PathBuf) -> Result<LeanResult> {
        if !self.available {
            return Ok(LeanResult::NotAvailable(
                "Lean 4 is not installed or not in PATH".to_string(),
            ));
        }

        // First, try to build the OVSM library if needed
        self.ensure_library_built()?;

        // Run lean with the library in the path
        let output = Command::new(&self.lean_path)
            .arg(file)
            .env("LEAN_PATH", self.get_lean_path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    Ok(LeanResult::Success)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let errors = self.parse_lean_errors(&stderr);
                    Ok(LeanResult::Errors(errors))
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    Ok(LeanResult::Timeout)
                } else {
                    Err(Error::compiler(format!("Failed to run Lean: {}", e)))
                }
            }
        }
    }

    /// Build the OVSM Lean library if needed
    fn ensure_library_built(&self) -> Result<()> {
        if !self.ovsm_lib_path.exists() {
            return Ok(()); // Library doesn't exist, will be created on first use
        }

        let lake_file = self.ovsm_lib_path.join("lakefile.lean");
        if !lake_file.exists() {
            return Ok(()); // Not a lake project
        }

        // Check if build is needed by looking at .lake directory
        let lake_dir = self.ovsm_lib_path.join(".lake");
        if lake_dir.exists() {
            return Ok(()); // Already built
        }

        // Build the library
        let output = Command::new(&self.lake_path)
            .arg("build")
            .current_dir(&self.ovsm_lib_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("Failed to build OVSM Lean library: {}", stderr);
                }
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to run lake build: {}", e);
                Ok(())
            }
        }
    }

    /// Get LEAN_PATH for imports
    fn get_lean_path(&self) -> String {
        let mut paths = vec![];

        // Add OVSM library
        if self.ovsm_lib_path.exists() {
            paths.push(self.ovsm_lib_path.display().to_string());

            // Add .lake/build/lib if it exists
            let build_lib = self.ovsm_lib_path.join(".lake").join("build").join("lib");
            if build_lib.exists() {
                paths.push(build_lib.display().to_string());
            }
        }

        paths.join(":")
    }

    /// Parse Lean error messages from stderr
    fn parse_lean_errors(&self, stderr: &str) -> Vec<LeanMessage> {
        let mut errors = Vec::new();
        // Simple pattern without lookahead - just match the first line of each error
        let error_re =
            regex::Regex::new(r"(?m)^(.+):(\d+):(\d+):\s*(error|warning|info):\s*(.+)$").unwrap();

        for cap in error_re.captures_iter(stderr) {
            let file = cap
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let line = cap
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let column = cap
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            let severity = cap.get(4).map(|m| m.as_str()).unwrap_or("error");
            let message = cap
                .get(5)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            errors.push(LeanMessage {
                file,
                line,
                column,
                severity: match severity {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    _ => Severity::Info,
                },
                message,
            });
        }

        // If no structured errors found but stderr isn't empty, create a generic error
        if errors.is_empty() && !stderr.trim().is_empty() {
            errors.push(LeanMessage {
                file: String::new(),
                line: 0,
                column: 0,
                severity: Severity::Error,
                message: stderr.trim().to_string(),
            });
        }

        errors
    }

    /// Run lake with JSON output
    #[allow(dead_code)]
    pub fn lake_build_json(&self) -> Result<LakeBuildResult> {
        if !self.available {
            return Ok(LakeBuildResult {
                success: false,
                errors: vec![],
                warnings: vec![],
            });
        }

        let output = Command::new(&self.lake_path)
            .args(["--json", "build"])
            .current_dir(&self.ovsm_lib_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| Error::compiler(format!("Failed to run lake: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON output
        if output.status.success() {
            Ok(LakeBuildResult {
                success: true,
                errors: vec![],
                warnings: vec![],
            })
        } else {
            // Try to parse JSON errors
            let errors = self.parse_lean_errors(&String::from_utf8_lossy(&output.stderr));
            Ok(LakeBuildResult {
                success: false,
                errors: errors
                    .into_iter()
                    .filter(|e| e.severity == Severity::Error)
                    .collect(),
                warnings: vec![],
            })
        }
    }
}

/// Result of Lean verification
#[derive(Debug, Clone)]
pub enum LeanResult {
    /// All verification conditions proved
    Success,
    /// Verification failed with errors
    Errors(Vec<LeanMessage>),
    /// Verification timed out
    Timeout,
    /// Lean is not available
    NotAvailable(String),
}

/// A message from Lean (error, warning, or info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanMessage {
    /// Source file
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Severity level
    pub severity: Severity,
    /// Message content
    pub message: String,
}

/// Message severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Error severity
    Error,
    /// Warning severity
    Warning,
    /// Info severity
    Info,
}

/// Error from Lean
#[derive(Debug, Clone)]
pub struct LeanError {
    /// Error message
    pub message: String,
    /// Source location
    pub location: Option<(String, usize, usize)>,
}

impl std::fmt::Display for LeanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some((file, line, col)) = &self.location {
            write!(f, "{}:{}:{}: {}", file, line, col, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for LeanError {}

/// Result of lake build
#[derive(Debug, Clone)]
pub struct LakeBuildResult {
    /// Whether build succeeded
    pub success: bool,
    /// Errors
    pub errors: Vec<LeanMessage>,
    /// Warnings
    pub warnings: Vec<LeanMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lean_errors() {
        let bridge = LeanBridge {
            lean_path: PathBuf::from("lean"),
            lake_path: PathBuf::from("lake"),
            ovsm_lib_path: PathBuf::from("."),
            timeout: Duration::from_secs(60),
            available: false,
        };

        let stderr = r#"test.lean:5:2: error: type mismatch
  rfl
has type
  f = f : Prop
but is expected to have type
  f = 2 : Prop
test.lean:10:0: warning: unused variable
"#;

        let errors = bridge.parse_lean_errors(stderr);
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].line, 5);
        assert_eq!(errors[0].column, 2);
        assert_eq!(errors[0].severity, Severity::Error);
        assert!(errors[0].message.contains("type mismatch"));
        assert_eq!(errors[1].severity, Severity::Warning);
    }

    #[test]
    fn test_lean_result_success() {
        let result = LeanResult::Success;
        assert!(matches!(result, LeanResult::Success));
    }
}
