# README media

The extension README uses these illustrated previews:

- `spellbook-banner.svg`: editable product artwork using the extension's book emblem and macro syntax colors.
- `autocomplete-walkthrough.gif`: an illustrated sequence populated by Spellbook's actual completion provider and character counter.
- `macro-samples.svg`: code cards populated from the shipped sample library.

PNG versions of the banner and sample cards are used in the extension README, since VS Code extension packaging restricts SVG images in README files.

These three assets are illustrations, not captures of the VS Code interface. The other PNG files in this directory are screenshots of the original desktop app.

To regenerate the animated walkthrough and sample cards, compile the extension and run `node scripts/readme-media.cjs` from the repository root. ImageMagick 7 must be on PATH; `MAGICK_BIN` can specify its executable. Intermediate frames go in the ignored `.capture-data/readme-media` directory. Edit the banner SVG directly.
