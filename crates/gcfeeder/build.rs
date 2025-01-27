#![deny(clippy::all)]
use std::env;
#[cfg(windows)]
use std::{
    fs::File,
    io::{self, prelude::*},
    path::Path,
};

pub fn main() {
    const VERSION_VAR: &str = "VERSION";
    println!("cargo:rerun-if-env-changed={VERSION_VAR}");

    let version = match env::var(VERSION_VAR) {
        Ok(v) => {
            if env::var("CI").is_ok() {
                let package_version = format!("v{}", env!("CARGO_PKG_VERSION"));
                if v != package_version {
                    panic!("Expected {VERSION_VAR} to be {package_version}, was {v}")
                }
            }
            v
        }
        Err(_) => "".to_string(),
    };

    println!("cargo:rustc-env={VERSION_VAR}={version}");

    {
        let mut emitter = vergen_git2::Emitter::default();
        emitter
            .add_instructions(
                &vergen_git2::Git2Builder::default()
                    .describe(true, true, None)
                    .build()
                    .expect("build git2 instruction"),
            )
            .expect("add git2 instruction");

        if emitter.emit().is_err() {
            println!("cargo:rustc-env={VERSION_VAR}=unknown");
            println!("cargo:rustc-env=VERGEN_GIT_DESCRIBE=");
        }
    }

    #[cfg(windows)]
    {
        const ICON_PATH: &str = "resource/icon.png";
        println!("cargo:rerun-if-changed={ICON_PATH}");

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

        embed_resource::compile(&rc_path, embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
