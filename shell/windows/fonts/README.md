# Tabular copies of Inter and Unbounded

The flyout's figures, badges, resets-in lines and detail lines are drawn with these copies, so
their digits are the same width, as the 1.x flyout's are. The toolkit has no setting for font
features, so the copies carry the feature as their default.

Each copy is its original (`src/Sobreiro/Fonts/`) with two changes:

- In the character map, every character the font's `tnum` feature replaces (the digits 0-9 and,
  in Inter, the punctuation and signs the feature also covers) shows the tabular glyph.
- The family, full, PostScript and unique names gain "Tabular" (`Inter Tabular`,
  `Unbounded Tabular`), so a copy and its original can be used side by side.

No glyph outline or metric changes. The fonts stay under the SIL Open Font License, with the
copyright notice of the originals: `OFL-Inter.txt` and `OFL-Unbounded.txt`.

Made with fontTools at the version in `scripts/requirements-fonts.txt`:

    python -I scripts/tabular-fonts.py          # writes the three fonts
    python -I scripts/tabular-fonts.py --check  # fails if they differ from what the script writes
