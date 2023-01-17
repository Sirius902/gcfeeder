#![deny(clippy::all)]
use std::{
    env,
    fs::File,
    io::{self, prelude::*},
    path::Path,
};

pub fn main() {
    {
        let mut config = vergen::Config::default();
        *config.git_mut().sha_kind_mut() = vergen::ShaKind::Short;

        vergen::vergen(config).unwrap();
    }

    const VERSION_VAR: &str = "VERSION";
    println!("cargo:rerun-if-env-changed={}", VERSION_VAR);

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
        Err(_) => "".to_string(),
    };

    println!("cargo:rustc-env={}={}", VERSION_VAR, version);

    #[cfg(windows)]
    {
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

        let rc_path = Path::new(&out_dir).join("app.rc");

        File::create(&rc_path)
            .and_then(|mut f| f.write_all("IDI_ICON1 ICON DISCARDABLE \"icon.ico\"".as_bytes()))
            .unwrap();

        embed_resource::compile(&rc_path);
    }
}
