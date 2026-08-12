//! Install the bundled agent skill into the user's local Claude Code skills.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};

static SKILLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/skills");

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("could not determine home directory (neither HOME nor USERPROFILE is set)")
}

fn extract_dir(dir: &Dir<'_>, destination: &Path) -> Result<usize> {
    std::fs::create_dir_all(destination)
        .with_context(|| format!("create directory {}", destination.display()))?;

    let mut count = 0;
    for file in dir.files() {
        let name = file.path().file_name().expect("embedded file has a name");
        let output = destination.join(name);
        std::fs::write(&output, file.contents())
            .with_context(|| format!("write {}", output.display()))?;
        count += 1;
    }
    for child in dir.dirs() {
        let name = child
            .path()
            .file_name()
            .expect("embedded directory has a name");
        count += extract_dir(child, &destination.join(name))?;
    }
    Ok(count)
}

pub(crate) fn install_skills() -> Result<()> {
    let destination = home_dir()?.join(".claude").join("skills");
    let count = extract_dir(&SKILLS_DIR, &destination)?;
    println!(
        "Installed {count} skill file(s) to {}",
        destination.display()
    );
    Ok(())
}
