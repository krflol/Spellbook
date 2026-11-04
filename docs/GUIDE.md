# Using Spellbook

## Write and organize

Choose **New macro**, or open **Snippets** and choose **New from template**. Every template includes a class, category tag, and usage notes under **Details**. Search the sample library by spell, class, name, or condition; class filters include universal helpers.

The left library searches names, body text, notes, tags, and characters. Favorites and class filters narrow the list. Open several macros in tabs and switch between them; close a tab without deleting the macro. **Move to trash** is in the macro's overflow menu. Recover it from **Trash** in the library.

Edits save after a short typing pause, on tab close, and on normal app close. **Ctrl+S** saves all open macros immediately. **History** shows the latest 100 saved revisions; restoring creates a new revision and preserves the current changes first. All revisions remain in the database.

If another Spellbook window changed the same macro, the draft remains open with an error. Duplicate it to preserve both versions, or explicitly reload the saved version. A failed macro save prevents normal window close. Keep regular JSON exports for backups.

## Editor tools

| Shortcut | Action |
| --- | --- |
| Ctrl+Space | Request contextual suggestions |
| Tab / Enter | Accept selected suggestion |
| Up / Down | Move through suggestions |
| Esc | Dismiss suggestions/find |
| Ctrl+F / Ctrl+H | Find / replace |
| Ctrl+G | Go to line |
| Ctrl+D | Duplicate current line |
| Ctrl+Z / Ctrl+Shift+Z | Undo / redo |
| Ctrl+N / Ctrl+W | New macro / close tab |
| Ctrl+O | Import files |
| Ctrl+S | Save all open macros |
| Ctrl+Shift+S | Export library |
| F1 | Help and shortcuts |

Font size and bracket pairing are available in the editor and **Workspace → Settings**. The app restores open tabs, active macro, cursor positions, code settings, panel visibility, and window size. At compact window sizes, **Tools** opens a floating panel.

**Problems** checks the 255-character budget, macro names, commands, conditions, brackets, missing arguments, and Lua delimiter balance. Click a problem to move to its location. Character count and UTF-8 bytes are displayed separately. Long drafts remain editable up to 64 KiB; the game still imposes its own macro limits. The checks do not execute Lua, fully parse Lua, verify learned spells, or simulate combat. Addon commands remain editable even when unrecognized.

## Stances and forms

The samples include Warrior stance selectors, Charge/Intercept, Overpower, Taunt, Pummel, stance-sensitive interrupts and cooldowns; Druid Bear/Cat switches, travel forms, Prowl, Dash, powershifting, caster healing, and form-sensitive attacks; Ghost Wolf, Shadowform healing, and Rogue stealth.

Run **Inspect forms** in game to print each learned form's slot, spell ID, and name. Use the first number in `[form:N]` or `[stance:N]`. The numbered samples assume Warrior Battle/Defensive/Berserker 1/2/3 and Druid Bear/Cat 1/3; check and adjust for your character. Select Dire Bear Form instead of Bear Form where appropriate. Each `/cast` branch chooses one action per press; a stance change may need a second press for the ability. Shifting still respects mana, rage, and cooldowns. An explicit powershift can leave you in caster form if the recast fails.

The samples are editable examples, not live-client-verified rotations. Reference reports: [Warrior stance macros](https://us.forums.blizzard.com/en/wow/t/classic-warrior-best-classic-macros/358678), [Druid powershifting](https://us.forums.blizzard.com/en/wow/t/working-powershift-macro/185358).

## Import, export, and sharing

**File → Import files** accepts UTF-8 text and versioned JSON libraries. You can also drop files onto the app, pass files on the command line, or paste text in **Import / export**. Imports create new documents. JSON imports are atomic per file, with limits of 8 MiB and 2,000 documents. Invalid libraries do not partially import. Imports from several files are processed individually.

Export the current macro as text or the whole non-trashed library as JSON. Exports ask for a new filename and never replace existing files. JSON preserves names, bodies, class/character labels, tags, notes, and favorites; revision history and trashed macros stay in the local database. Copy the data folder while Spellbook is closed for a complete backup including history.

**Copy macro** copies just the body. **Copy review prompt** in the overflow menu packages the macro and its notes for an assistant of your choice; it only copies text to your clipboard.

Spellbook imports `forever-atlas-macros` version 1 libraries and exports `spellbook-library` version 1. Updated Forever Atlas accepts both formats. Imported libraries use the current local workspace regardless of their original build label.

## Storage

| File | Contents |
| --- | --- |
| `library.sqlite3` | Macros, archived documents, all saved revisions |
| `session.json` | Open tabs, editor preferences, window size |
| `catalogue.json` | Optional custom completion names |

The default Windows folder is `%LOCALAPPDATA%\Spellbook`. `--portable` uses `data` beside the executable; `--data-dir PATH` overrides it. Those two flags cannot be combined. No admin rights are required; choose a writable folder for portable mode.
