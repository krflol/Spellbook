use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenTab {
    pub id: i64,
    pub cursor: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub tabs: Vec<OpenTab>,
    pub active: Option<i64>,
    pub font_size: f32,
    pub auto_pairs: bool,
    pub tools_visible: bool,
    pub problems_open: bool,
    pub window_size: [f32; 2],
}
impl Default for Session {
    fn default() -> Self {
        Self {
            tabs: vec![],
            active: None,
            font_size: 16.0,
            auto_pairs: true,
            tools_visible: true,
            problems_open: true,
            window_size: [1480.0, 940.0],
        }
    }
}
impl Session {
    pub fn load(path: &Path) -> Result<Self> {
        let mut session: Self = serde_json::from_str(&read_limited(path, 1024 * 1024)?)?;
        ensure!(session.tabs.len() <= 2000, "Too many saved editor tabs");
        session.font_size = session.font_size.clamp(11.0, 28.0);
        session.window_size[0] = session.window_size[0].clamp(960.0, 3840.0);
        session.window_size[1] = session.window_size[1].clamp(640.0, 2160.0);
        Ok(session)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        write_atomic(path, serde_json::to_string_pretty(self)?.as_bytes())
    }
}

pub fn data_dir(explicit: Option<PathBuf>, portable: bool) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if portable {
        return Ok(std::env::current_exe()?
            .parent()
            .context("Locate executable directory")?
            .join("data"));
    }
    #[cfg(target_os = "windows")]
    if let Some(path) = std::env::var_os("LOCALAPPDATA") {
        return Ok(PathBuf::from(path).join("Spellbook"));
    }
    #[cfg(target_os = "macos")]
    if let Some(path) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(path).join("Library/Application Support/Spellbook"));
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
            return Ok(PathBuf::from(path).join("spellbook"));
        }
        if let Some(path) = std::env::var_os("HOME") {
            return Ok(PathBuf::from(path).join(".local/share/spellbook"));
        }
    }
    anyhow::bail!(
        "Could not locate the user data directory. Start with --data-dir PATH or --portable."
    )
}

pub fn read_limited(path: &Path, limit: usize) -> Result<String> {
    let file = std::fs::File::open(path).with_context(|| format!("Open {}", path.display()))?;
    ensure!(
        file.metadata()?.len() <= limit as u64,
        "File exceeds the {} byte limit",
        limit
    );
    let mut text = String::new();
    file.take(limit as u64 + 1).read_to_string(&mut text)?;
    ensure!(text.len() <= limit, "File exceeds the {} byte limit", limit);
    Ok(text)
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let pending = path.with_extension(format!("{}.pending", std::process::id()));
    let result = (|| -> Result<()> {
        let mut file = std::fs::File::create(&pending)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&pending, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&pending);
    }
    result.with_context(|| format!("Save {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sessions_replace_atomically_and_bound_invalid_sizes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        let mut session = Session::default();
        session.save(&path).unwrap();
        session.tabs.push(OpenTab { id: 17, cursor: 32 });
        session.active = Some(17);
        session.save(&path).unwrap();
        assert_eq!(session, Session::load(&path).unwrap());
        session.font_size = 900.0;
        session.save(&path).unwrap();
        assert_eq!(Session::load(&path).unwrap().font_size, 28.0);
        assert!(read_limited(&path, 1).is_err());
    }
}
