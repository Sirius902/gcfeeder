#![deny(clippy::all)]
use std::{
    env,
    fs::File,
    io::{self, Write},
    path::Path,
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

    const ICON_PATH: &str = "resource/icon.png";
    println!("cargo:rerun-if-changed={}", ICON_PATH);

    let icon = image::load(
        io::BufReader::new(File::open(ICON_PATH).unwrap()),
        image::ImageFormat::Png,
    )
    .unwrap();

    let out_dir = env::var("OUT_DIR").unwrap();
    let ico_path = Path::new(&out_dir).join("icon.ico");

    icon.resize(256, 256, image::imageops::CatmullRom)
        .save_with_format(ico_path, image::ImageFormat::Ico)
        .unwrap();

    #[cfg(windows)]
    {
        let rc_path = Path::new(&out_dir).join("app.rc");

        File::create(&rc_path)
            .and_then(|mut f| f.write_all("IDI_ICON1 ICON DISCARDABLE \"icon.ico\"".as_bytes()))
            .unwrap();

        embed_resource::compile(&rc_path);
    }
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
