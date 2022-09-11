#![deny(clippy::all)]
use std::{
    env, io,
    process::{Command, Stdio},
};

use regex::Regex;

const VERSION_VAR: &str = "VERSION";

pub fn main() {
    let regular_semver = Regex::new(r"^v\d+(\.\d+)?{2}$").unwrap();
    let version = match env::var(VERSION_VAR) {
        Ok(v) => {
            let package_version = format!("v{}", env!("CARGO_PKG_VERSION"));
            if regular_semver.is_match(&v) && v != package_version {
                panic!(
                    "Expected {} to be {}, was {}",
                    VERSION_VAR, package_version, v
                )
            }
            v
        }
        Err(_) => BuildInfo::from_git().unwrap_or_default().version,
    };

    println!("cargo:rustc-env={}={}", VERSION_VAR, version);
}

struct BuildInfo {
    version: String,
}

impl BuildInfo {
    pub fn from_git() -> io::Result<Self> {
        let version_output = Command::new("git")
            .args(["describe", "--always", "--dirty", "--tags"])
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()?;

        version_output
            .status
            .code()
            .and_then(|c| if c == 0 { Some(()) } else { None })
            .expect("failed to execute version command");

        let version = String::from_utf8_lossy(&version_output.stdout).to_string();

        Ok(Self { version })
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            version: "unknown".to_owned(),
        }
    }
}
