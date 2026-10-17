//! Finding the player's Burnout Paradise Remastered: the folder given, else
//! the one the GTA Paradise setup remembered (if it's on this PC), else the
//! usual install places (Steam libraries, EA app / Origin defaults). Only
//! reads; never changes anything.

use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// Every Steam library folder (from Steam's libraryfolders.vdf).
fn steam_libraries() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for steam in [r"C:\Program Files (x86)\Steam", r"C:\Program Files\Steam"] {
        let Ok(text) = std::fs::read_to_string(Path::new(steam).join(r"steamapps\libraryfolders.vdf")) else { continue };
        out.push(PathBuf::from(steam));
        for line in text.lines() {
            let parts: Vec<&str> = line.split('"').collect();
            // 	"path"		"D:\\SteamLibrary"
            if parts.len() >= 4 && parts[1] == "path" {
                out.push(PathBuf::from(parts[3].replace(r"\\", r"\")));
            }
        }
    }
    out.dedup();
    out
}

/// Burnout Paradise Remastered (PC): BurnoutPR.exe and WORLDCOL.BIN.
pub fn check_burnout(p: &Path) -> Result<PathBuf> {
    if !p.join("BurnoutPR.exe").is_file() || !p.join("WORLDCOL.BIN").is_file() {
        bail!("not Burnout Paradise Remastered (no BurnoutPR.exe / WORLDCOL.BIN here)");
    }
    Ok(p.to_path_buf())
}

/// The Burnout folder the GTA Paradise setup remembered
/// (%LOCALAPPDATA%\GTA-Paradise\setup.json), if it ran on this PC.
fn gta_setup_burnout() -> Option<PathBuf> {
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let config: serde_json::Value = serde_json::from_slice(&std::fs::read(local.join("GTA-Paradise").join("setup.json")).ok()?).ok()?;
    config["burnout"].as_str().map(PathBuf::from)
}

fn candidates() -> Vec<PathBuf> {
    let mut c: Vec<PathBuf> = gta_setup_burnout().into_iter().collect();
    c.extend(steam_libraries().into_iter().map(|l| l.join(r"steamapps\common\Burnout Paradise Remastered")));
    for base in [r"C:\Program Files\EA Games", r"C:\Program Files (x86)\Origin Games", r"C:\Program Files\Origin Games"] {
        c.push(Path::new(base).join("Burnout Paradise Remastered"));
    }
    c
}

/// The first of `given` and the places above that is Burnout.
pub fn detect_burnout(given: Option<PathBuf>) -> Option<PathBuf> {
    given.into_iter().chain(candidates()).find_map(|c| check_burnout(&c).ok())
}
