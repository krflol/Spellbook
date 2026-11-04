use spellbook::macros::{store::Store, LibraryFile, MacroDocument};

#[test]
fn atlas_imports_are_normalized_and_spellbook_exports_round_trip() {
    let atlas = r##"{"format":"forever-atlas-macros","version":1,"build":"wowf:test","macros":[{"name":"猫 form","body":"#showtooltip\r\n/cast !Cat Form","class":"Druid","notes":"Keep my notes","favorite":true}]}"##;
    let imported = LibraryFile::decode(atlas).unwrap();
    assert!(!imported.macros[0].body.contains('\r'));
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("library.sqlite3"), "wow-forever").unwrap();
    store.import(&imported.macros).unwrap();
    let stored: Vec<MacroDocument> = store
        .list()
        .unwrap()
        .into_iter()
        .map(|s| s.document)
        .collect();
    let exported = LibraryFile::encode("wow-forever", stored.clone()).unwrap();
    assert!(exported.contains("spellbook-library"));
    assert_eq!(LibraryFile::decode(&exported).unwrap().macros, stored);
    assert!(LibraryFile::decode(&exported.replace("\"version\": 1", "\"version\": 99")).is_err());
}
