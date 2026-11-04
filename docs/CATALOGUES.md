# Autocomplete catalogues

Spellbook includes a small curated English vocabulary. It is a reference list, not a complete client database or your character's spellbook. Commands, conditions, targets, and Lua API references are available independently of spell/item names.

Import a JSON file from **Workspace → Settings → Import catalogue**. It replaces the previous custom catalogue and merges with bundled names; a duplicate of the same kind/name replaces that suggestion. **Reset to bundled names** removes the custom file. The imported file is copied into your data folder, so the original does not need to remain accessible.

```json
{
  "format": "spellbook-catalogue",
  "version": 1,
  "names": [
    { "name": "Fireball", "kind": "spell", "id": 133 },
    { "name": "Healing Potion", "kind": "item", "id": 929 },
    { "name": "Éclat lunaire", "kind": "spell" }
  ]
}
```

`kind` is `spell` or `item`; `id` is optional and must be nonnegative. Spellbook inserts names, not IDs. Use the exact spelling your client accepts. Files are limited to 8 MiB and 50,000 names. Each trimmed name must contain 1–256 UTF-8 bytes and no control characters. Unknown fields and unsupported versions are rejected before the installed catalogue changes.

An example file is included at [`assets/catalogue-example.json`](../assets/catalogue-example.json). No game installation or internet lookup is required by the app.
