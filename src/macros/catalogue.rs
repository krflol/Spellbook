//! Small, offline reference vocabulary; users can merge their own client names.
use super::language::{Catalogue, Symbol, SymbolKind};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueFile {
    pub format: String,
    pub version: u32,
    pub names: Vec<Name>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Name {
    pub name: String,
    pub kind: NameKind,
    #[serde(default)]
    pub id: i64,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NameKind {
    Spell,
    Item,
}

impl CatalogueFile {
    pub fn decode(text: &str) -> Result<Self> {
        ensure!(text.len() <= 8 * 1024 * 1024, "Catalogue exceeds 8 MiB");
        let mut file: Self = serde_json::from_str(text.trim_start_matches('\u{feff}'))?;
        ensure!(
            file.format == "spellbook-catalogue" && file.version == 1,
            "Unsupported catalogue format or version"
        );
        ensure!(file.names.len() <= 50_000, "Catalogue exceeds 50,000 names");
        for entry in &mut file.names {
            entry.name = entry.name.trim().to_owned();
            ensure!(
                !entry.name.is_empty()
                    && entry.name.len() <= 256
                    && !entry.name.chars().any(char::is_control),
                "Names must be 1–256 bytes and contain no control characters"
            );
            ensure!(entry.id >= 0, "Spell and item IDs cannot be negative");
        }
        Ok(file)
    }
    pub fn read(path: &Path) -> Result<Self> {
        Self::decode(
            &crate::session::read_limited(path, 8 * 1024 * 1024)
                .context("Read autocomplete catalogue")?,
        )
    }
}

impl Catalogue {
    pub fn bundled() -> Self {
        let names = include_str!("../../assets/names.txt")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| line.split_once('|'))
            .map(|(kind, name)| Symbol {
                name: name.into(),
                folded: name.to_lowercase(),
                id: 0,
                kind: if kind == "item" {
                    SymbolKind::Item
                } else {
                    SymbolKind::Spell
                },
            })
            .collect();
        Self { symbols: names }
    }
    /// Replace duplicate names in place and preserve all unrelated suggestions.
    pub fn merge(&mut self, file: &CatalogueFile) {
        let mut names: BTreeMap<(bool, String), Symbol> = self
            .symbols
            .drain(..)
            .map(|s| ((s.kind == SymbolKind::Item, s.folded.clone()), s))
            .collect();
        for entry in &file.names {
            let kind = match entry.kind {
                NameKind::Spell => SymbolKind::Spell,
                NameKind::Item => SymbolKind::Item,
            };
            let folded = entry.name.to_lowercase();
            names.insert(
                (kind == SymbolKind::Item, folded.clone()),
                Symbol {
                    name: entry.name.clone(),
                    folded,
                    kind,
                    id: entry.id,
                },
            );
        }
        self.symbols = names.into_values().collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_names_merge_and_replace_without_duplicates() {
        let mut c = Catalogue::bundled();
        let file = CatalogueFile::decode(r#"{"format":"spellbook-catalogue","version":1,"names":[{"name":"Fireball","kind":"spell","id":133},{"name":"Éclat lunaire","kind":"spell"}]}"#).unwrap();
        c.merge(&file);
        c.merge(&file);
        assert_eq!(c.symbols.iter().filter(|s| s.name == "Fireball").count(), 1);
        assert!(super::super::language::complete("/cast Éc", 8, &c)
            .iter()
            .any(|s| s.label == "Éclat lunaire"));
        assert!(c
            .symbols
            .iter()
            .any(|s| s.name == "Fireball" && s.id == 133));
        assert!(CatalogueFile::decode(
            r#"{"format":"spellbook-catalogue","version":2,"names":[]}"#
        )
        .is_err());
        assert!(CatalogueFile::decode(r#"{"format":"spellbook-catalogue","version":1,"names":[{"name":"bad\nname","kind":"spell"}]}"#).is_err());
    }
}
