use std::env;

fn main() {
    emit_version("GCFEEDER_VERSION");

    #[cfg(windows)]
    embed_icon();
}

fn emit_version(version_env: &str) {
    println!("cargo:rerun-if-env-changed={version_env}");

    #[allow(clippy::unnecessary_lazy_evaluations)]
    let version = env::var(version_env)
        .ok()
        .or_else(|| {
            #[cfg(feature = "git-version")]
            {
                emit_git_version().ok()
            }

            #[cfg(not(feature = "git-version"))]
            {
                None
            }
        })
        .unwrap_or_else(|| format!("v{}-dev", env!("CARGO_PKG_VERSION")));

    println!("cargo:rustc-env={version_env}={version}");
}

#[cfg(feature = "git-version")]
fn emit_git_version() -> Result<String, git2::Error> {
    let repo = git2::Repository::discover(env!("CARGO_MANIFEST_DIR"))?;
    let git_dir = repo.path().join(".git");

    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());

    let head = repo.head()?;
    let r#ref = head.name().expect("ref name to be valid UTF-8");

    println!("cargo:rerun-if-changed={}", git_dir.join(r#ref).display());

    let commit = head.peel_to_commit()?;
    let short_id = commit.as_object().short_id()?;
    let short_hash = short_id
        .as_str()
        .expect("short commit hash to be valid UTF-8");

    let statuses = repo.statuses(Some(git2::StatusOptions::new().include_untracked(false)))?;
    let is_dirty = !statuses.is_empty();

    Ok(format!(
        "v{}-{}{}",
        env::var("CARGO_PKG_VERSION").unwrap(),
        short_hash,
        if is_dirty { "-dirty" } else { "" }
    ))
}

#[cfg(windows)]
fn embed_version() {
    use std::{
        fs::File,
        io::{self, prelude::*},
        path::Path,
    };

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
