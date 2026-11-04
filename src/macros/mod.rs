//! Offline WoW macro authoring. Text is never executed or written to the game client.
pub mod catalogue;
pub mod language;
pub mod snippets;
pub mod store;

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

pub const BODY_LIMIT: usize = 255;
pub const DRAFT_LIMIT: usize = 65_536;
pub const CLASSES: &[&str] = &[
    "Any class",
    "Druid",
    "Hunter",
    "Mage",
    "Paladin",
    "Priest",
    "Rogue",
    "Shaman",
    "Warlock",
    "Warrior",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MacroDocument {
    pub name: String,
    pub body: String,
    pub class: String,
    /// Empty means account-wide; otherwise a personal character label.
    pub character: String,
    pub tags: String,
    pub notes: String,
    pub favorite: bool,
}
impl Default for MacroDocument {
    fn default() -> Self {
        Self {
            name: "Untitled macro".into(),
            body: String::new(),
            class: "Any class".into(),
            character: String::new(),
            tags: String::new(),
            notes: String::new(),
            favorite: false,
        }
    }
}
impl MacroDocument {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.body.len() <= DRAFT_LIMIT, "Macro draft exceeds 64 KiB");
        ensure!(self.name.len() <= 512, "Macro name exceeds 512 bytes");
        ensure!(self.notes.len() <= DRAFT_LIMIT, "Notes exceed 64 KiB");
        ensure!(
            self.tags.len() <= 2048 && self.character.len() <= 512 && self.class.len() <= 128,
            "Macro metadata is too long"
        );
        ensure!(
            !self.body.contains('\0'),
            "Macro text contains a NUL character"
        );
        Ok(())
    }
}

/// Normalize pasted Windows line endings, without changing executable text.
pub fn normalize(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

pub fn byte_index(text: &str, character: usize) -> usize {
    text.char_indices()
        .nth(character)
        .map_or(text.len(), |(i, _)| i)
}

pub fn char_index(text: &str, byte: usize) -> usize {
    text.char_indices().take_while(|(i, _)| *i < byte).count()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryFile {
    pub format: String,
    pub version: u32,
    pub build: String,
    pub macros: Vec<MacroDocument>,
}
impl LibraryFile {
    pub fn encode(build: &str, macros: Vec<MacroDocument>) -> Result<String> {
        Ok(serde_json::to_string_pretty(&Self {
            format: "spellbook-library".into(),
            version: 1,
            build: build.into(),
            macros,
        })?)
    }
    pub fn decode(text: &str) -> Result<Self> {
        ensure!(text.len() <= 8 * 1024 * 1024, "Import exceeds 8 MiB");
        let mut library: Self = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
        ensure!(
            matches!(
                library.format.as_str(),
                "spellbook-library" | "forever-atlas-macros"
            ) && library.version == 1,
            "Unsupported macro library format or version"
        );
        ensure!(
            library.macros.len() <= 2000,
            "Import contains more than 2,000 macros"
        );
        for doc in &mut library.macros {
            doc.body = normalize(&doc.body);
            doc.validate()?;
        }
        Ok(library)
    }
}
