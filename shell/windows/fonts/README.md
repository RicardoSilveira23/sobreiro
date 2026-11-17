# Tabular copies of Inter and Unbounded

The flyout's figures, badges, resets-in lines and detail lines are drawn with these copies, so
their digits are the same width, as the 1.x flyout's are. The toolkit has no setting for font
features, so the copies carry the feature as their default.

Each copy is its original, in this folder too, with two changes:

- In the character map, every character the font's `tnum` feature replaces shows the tabular
  glyph: the digits 0-9 and, in Inter, the space and the punctuation and signs the feature also
  covers. The colon is left as it was: Inter's `calt` feature raises a colon between digits, and a
  colon that already shows its tabular glyph matches none of its rules, so a time would sit lower
  than in the original.
- The family, full, PostScript and unique names gain "Tabular" (`Inter Tabular`,
  `Unbounded Tabular`), so a copy and its original can be used side by side.

No glyph outline or metric changes. The fonts stay under the SIL Open Font License, with the
copyright notice of the originals: `OFL-Inter.txt` and `OFL-Unbounded.txt`.

Made with fontTools at the version in `scripts/requirements-fonts.txt`:

    python -I scripts/tabular-fonts.py          # writes the three fonts
    python -I scripts/tabular-fonts.py --check  # fails if they differ from what the script writes
