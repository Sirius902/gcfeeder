#![deny(clippy::all)]
use std::{
    env, io,
    process::{Command, Stdio},
};

const VERSION_VAR: &str = "VERSION";

pub fn main() {
    let version = match env::var(VERSION_VAR) {
        Ok(v) => {
            let package_version = format!("v{}", env!("CARGO_PKG_VERSION"));
            if v != package_version {
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
            .args(["rev-parse", "--short=7", "HEAD"])
            .stdout(Stdio::piped())
            .spawn()?
            .wait_with_output()?;

        version_output
            .status
            .code()
            .filter(|c| *c == 0)
            .expect("failed to execute version command");

        let version = format!("g{}", String::from_utf8_lossy(&version_output.stdout));

        Ok(Self { version })
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self {
            version: "unknown version".to_owned(),
        }
    }
}
