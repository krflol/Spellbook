use super::MacroDocument;
use anyhow::{ensure, Result};
use rusqlite::{params, Connection};
use std::{path::Path, time::Duration};

#[derive(Clone, Debug)]
pub struct SavedMacro {
    pub id: i64,
    pub revision: i64,
    pub document: MacroDocument,
    pub archived: bool,
    pub updated_at: String,
}
#[derive(Clone)]
pub struct Revision {
    pub number: i64,
    pub document: MacroDocument,
    pub at: String,
}
pub struct Store {
    conn: Connection,
    build: String,
}
impl Store {
    pub fn open(path: &Path, build: &str) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(2))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS spellbook_macros (
            id INTEGER PRIMARY KEY, build TEXT NOT NULL, revision INTEGER NOT NULL,
            document TEXT NOT NULL, archived INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f','now')));
            CREATE INDEX IF NOT EXISTS spellbook_macros_build ON spellbook_macros(build,archived);
            CREATE TABLE IF NOT EXISTS spellbook_revisions (
            macro_id INTEGER NOT NULL, revision INTEGER NOT NULL, document TEXT NOT NULL,
            saved_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f','now')),
            PRIMARY KEY(macro_id,revision));",
        )?;
        Ok(Self {
            conn,
            build: build.into(),
        })
    }
    pub fn list(&self) -> Result<Vec<SavedMacro>> {
        let mut stmt = self.conn.prepare("SELECT id,revision,document,archived,updated_at FROM spellbook_macros WHERE build=?1 ORDER BY updated_at DESC,id DESC")?;
        let rows = stmt.query_map([&self.build], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get::<_, String>(2)?,
                r.get(3)?,
                r.get(4)?,
            ))
        })?;
        rows.map(|row| {
            let (id, revision, json, archived, updated_at) = row?;
            Ok(SavedMacro {
                id,
                revision,
                document: serde_json::from_str(&json)?,
                archived,
                updated_at,
            })
        })
        .collect()
    }
    pub fn create(&mut self, doc: &MacroDocument) -> Result<SavedMacro> {
        let mut imported = self.import(std::slice::from_ref(doc))?;
        Ok(imported.remove(0))
    }
    /// All or nothing, always as new records. Imported IDs cannot overwrite local work.
    pub fn import(&mut self, docs: &[MacroDocument]) -> Result<Vec<SavedMacro>> {
        for doc in docs {
            doc.validate()?;
        }
        let tx = self.conn.transaction()?;
        let mut ids = Vec::new();
        for doc in docs {
            let json = serde_json::to_string(doc)?;
            tx.execute(
                "INSERT INTO spellbook_macros(build,revision,document) VALUES (?1,1,?2)",
                params![self.build, json],
            )?;
            let id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO spellbook_revisions(macro_id,revision,document) VALUES (?1,1,?2)",
                params![id, json],
            )?;
            ids.push(id);
        }
        tx.commit()?;
        Ok(self
            .list()?
            .into_iter()
            .filter(|s| ids.contains(&s.id))
            .collect())
    }
    pub fn save(&mut self, saved: &mut SavedMacro, doc: &MacroDocument) -> Result<()> {
        if &saved.document == doc {
            return Ok(());
        }
        doc.validate()?;
        let json = serde_json::to_string(doc)?;
        let tx = self.conn.transaction()?;
        let changed = tx.execute("UPDATE spellbook_macros SET document=?1,revision=revision+1,updated_at=strftime('%Y-%m-%d %H:%M:%f','now') WHERE id=?2 AND build=?3 AND revision=?4 AND archived=0", params![json,saved.id,self.build,saved.revision])?;
        ensure!(changed == 1, "This macro changed in another Spellbook window. Your draft is retained; duplicate it to keep both versions, or reload the saved version.");
        tx.execute(
            "INSERT INTO spellbook_revisions(macro_id,revision,document) VALUES (?1,?2,?3)",
            params![saved.id, saved.revision + 1, json],
        )?;
        tx.commit()?;
        saved.revision += 1;
        saved.document = doc.clone();
        Ok(())
    }
    pub fn archive(&mut self, saved: &SavedMacro, archived: bool) -> Result<()> {
        let changed = self.conn.execute("UPDATE spellbook_macros SET archived=?1,revision=revision+1,updated_at=strftime('%Y-%m-%d %H:%M:%f','now') WHERE id=?2 AND build=?3 AND revision=?4", params![archived,saved.id,self.build,saved.revision])?;
        ensure!(
            changed == 1,
            "Macro changed in another Spellbook window; refresh the library first"
        );
        Ok(())
    }
    pub fn revisions(&self, id: i64) -> Result<Vec<Revision>> {
        let mut stmt = self.conn.prepare("SELECT r.revision,r.document,r.saved_at FROM spellbook_revisions r JOIN spellbook_macros m ON m.id=r.macro_id WHERE m.build=?1 AND m.id=?2 ORDER BY r.revision DESC LIMIT 100")?;
        let rows = stmt.query_map(params![self.build, id], |r| {
            Ok((r.get(0)?, r.get::<_, String>(1)?, r.get(2)?))
        })?;
        rows.map(|row| {
            let (number, json, at) = row?;
            Ok(Revision {
                number,
                document: serde_json::from_str(&json)?,
                at,
            })
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_scopes_revisions_and_prevents_lost_updates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("desktop.sqlite3");
        let mut store = Store::open(&path, "a").unwrap();
        let mut saved = store.create(&MacroDocument::default()).unwrap();
        let stale = saved.clone();
        let doc = MacroDocument {
            body: "/cast Fireball".into(),
            ..saved.document.clone()
        };
        store.save(&mut saved, &doc).unwrap();
        let mut other = Store::open(&path, "a").unwrap();
        assert!(other
            .save(
                &mut stale.clone(),
                &MacroDocument {
                    body: "/cast Frostbolt".into(),
                    ..doc.clone()
                }
            )
            .is_err());
        assert_eq!(other.list().unwrap()[0].document.body, doc.body);
        assert!(Store::open(&path, "b").unwrap().list().unwrap().is_empty());
        assert_eq!(store.revisions(saved.id).unwrap().len(), 2);
        store.archive(&saved, true).unwrap();
        let archived = store.list().unwrap().remove(0);
        assert!(archived.archived);
        store.archive(&archived, false).unwrap();
        let mut restored = store.list().unwrap().remove(0);
        store
            .save(&mut restored, &MacroDocument::default())
            .unwrap();
        assert_eq!(store.revisions(saved.id).unwrap().len(), 3);
    }
    #[test]
    fn import_is_atomic_and_never_replaces_existing_macros() {
        let mut store = Store::open(Path::new(":memory:"), "test").unwrap();
        let doc = MacroDocument::default();
        store.create(&doc).unwrap();
        let bad = MacroDocument {
            body: "x".repeat(super::super::DRAFT_LIMIT + 1),
            ..doc.clone()
        };
        assert!(store.import(&[doc.clone(), bad]).is_err());
        assert_eq!(store.list().unwrap().len(), 1);
        store.import(&[doc.clone(), doc]).unwrap();
        assert_eq!(store.list().unwrap().len(), 3);
    }
}
