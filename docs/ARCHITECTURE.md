# Architecture

Spellbook is a native Rust application using egui/eframe with the glow renderer. All authoring and storage work offline. It has no dependency on Forever Atlas, a game process, a client extraction, or a remote AI service.

- `src/macros/language.rs`: Unicode-safe token ranges, syntax highlighting, completion contexts, structural diagnostics, and reference material.
- `src/macros/snippets.rs`: sample definitions, class/category metadata, form caveats, search.
- `src/macros/catalogue.rs`: bundled vocabulary and validated custom catalogue merging.
- `src/macros/store.rs`: scoped SQLite library, transactional imports, revision checks, recoverable archive, historical documents.
- `src/desktop/macro_editor.rs`: source editor and keyboard/mouse workflows; programmatic edits enter native undo history.
- `src/desktop/macros_view.rs`: library, tabs, samples, reference, metadata, history, transfers.
- `src/desktop/mod.rs`: native app lifecycle, menus, preferences, file drops, screenshot mode.
- `src/session.rs`: bounded file reads, atomic settings replacement, default data locations.

Native input tests cover actual egui keyboard, mouse, completion, pairing, selection, replacement, undo, library controls, and persistence. SQLite tests check atomic imports and concurrent revisions. Additional tests cover catalogue validation, session restoration, and cross-app transfer formats.

Developer screenshots use `spellbook.exe --capture-dir PATH`; this selects isolated data under `PATH/.capture-data`, renders eight states, and exits. Use a new directory for each run. The developer mode creates example macros only in that isolated library. Screenshots under `docs/screenshots` are real app captures; `docs/design/ui-concept.png` is an imagegen design artifact.

The minimum window size is 960×640. Side tools become a floating window below 1120 logical pixels. The packaged release targets Windows x64. Platform data-directory fallbacks exist for macOS/Linux, but those builds are not certified by this release.
