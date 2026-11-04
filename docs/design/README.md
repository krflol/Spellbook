# Spellbook visual design

The visual direction and final book emblem were generated with the built-in imagegen tool. The final prompts are preserved in [PROMPTS.md](PROMPTS.md).

- [UI concept](ui-concept.png): an exploratory design board for the editor and sample library. It is a mockup, not a screenshot.
- [Application emblem](../../assets/spellbook.png): the final transparent PNG used by the app header and native window icon. The Windows build converts it into an ICO resource while preserving alpha.
- [Desktop editor screenshot](../screenshots/editor.png): an actual capture from the original desktop application.
- [Extension README media](../screenshots/README.md): illustrated sample cards and an animated walkthrough populated from the extension's actual autocomplete results.

The desktop implementation carries over the charcoal, antique gold, teal selections, three-column editor, autocomplete popover, sample library, and character-budget indicator. Native menus and collapsible tool panels keep the interface compact; the app does not reproduce every decorative element of the concept. The VS Code extension uses VS Code's native interface and the user's editor theme; its README illustrations share Spellbook's branding.

Palette: background #11161C, panels #171E26, cards #1E2730, gold #E1BC70, text #E3E6E6, muted #97A6B1, teal #64CFB9. Blue and purple distinguish source tokens. No Blizzard artwork is included.
