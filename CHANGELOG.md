# Changelog

## 0.2.0

Spellbook is now available as a VS Code extension. Install the VSIX from GitHub Releases.

- Native macro editing with WoW/Lua highlighting, contextual IntelliSense, hover help, Outline, diagnostics, quick fixes, and character/byte budgets.
- Local library with autosave, metadata, favorites, Markdown notes, recoverable Trash, and cross-window edit protection.
- Revision comparisons and restoration through VS Code's native editor.
- 36 samples and snippets, Atlas/Spellbook library import/export, and custom autocomplete catalogues.
- Windows and Linux extension checks, real VS Code integration tests, and universal VSIX packaging.

The desktop application and its v0.1.0 download remain available. See [extension documentation](extension/README.md) for installation and migration.

## 0.1.0

First standalone Windows release, extracted from the Forever Atlas macro workshop.

- Native macro/Lua editor with contextual autocomplete, structural problems, hover help, bracket matching, and editing shortcuts.
- Local library with autosave, history, favorites, tags, class/character labels, and recoverable trash.
- 36 editable samples, including stance dancing, shapeshifting, and form-sensitive actions.
- Native file dialogs, drag-and-drop imports, JSON/text exports, Atlas library compatibility.
- Offline spell/item vocabulary, custom catalogue imports, restored editor sessions.
- Spellbook branding, generated visual design, actual native screenshots, portable Windows package, and CI checks.

Known limits: samples are not live-game-tested; diagnostics do not execute or fully parse Lua. Bundled names are a curated English subset. Windows binaries are unsigned; macOS/Linux builds are not tested in this release.
