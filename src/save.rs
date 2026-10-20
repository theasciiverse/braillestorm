//! The high score, in the player's data folder: `$XDG_DATA_HOME/braillestorm`
//! (`~/.local/share/braillestorm` without it), `%APPDATA%\braillestorm` on
//! Windows.

use std::path::PathBuf;

fn path() -> Option<PathBuf> {
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let base = if cfg!(windows) { env("APPDATA") } else { env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share"))) };
    Some(base?.join("braillestorm").join("hiscore"))
}

pub fn load_hi() -> u64 {
    path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| s.trim().parse().ok()).unwrap_or(0)
}

pub fn save_hi(score: u64) {
    let Some(p) = path() else { return };
    let result = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, format!("{score}\n")));
    match result {
        Ok(()) => log::info!("high score {score} saved to {}", p.display()),
        Err(e) => log::warn!("high score not saved to {}: {e}", p.display()),
    }
}
