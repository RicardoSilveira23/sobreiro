"""Makes the flyout's tabular copies of Inter and Unbounded in shell/windows/fonts/.

Each copy is its original with one change in the character map: every character whose default
glyph the font's `tnum` feature (default language system) replaces now shows the tabular glyph.
That is the digits 0-9 and the punctuation the feature also covers. The family, full, PostScript
and unique names gain "Tabular", so a copy can be imported beside its original. No glyph outline
or metric changes, and the head table's timestamps are kept, so a rerun writes the same bytes.

    python -I scripts/tabular-fonts.py          writes the three copies
    python -I scripts/tabular-fonts.py --check  rebuilds them in memory; exits 1 if the files on
                                                disk differ, if 0-9 differ in advance, or if any
                                                glyph differs from the original's

Needs fontTools at the version in scripts/requirements-fonts.txt, installed into the interpreter
itself (`python -m pip install --no-user -r scripts/requirements-fonts.txt`): `-I` does not see a
`--user` install.
"""
import io
import sys
from pathlib import Path

from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent.parent
ORIGINALS = ROOT / "src" / "Sobreiro" / "Fonts"
OUT = ROOT / "shell" / "windows" / "fonts"

# (original file, derived file, original family, derived family)
FONTS = [
    ("Inter-Regular.ttf", "InterTabular-Regular.ttf", "Inter", "Inter Tabular"),
    ("Inter-Medium.ttf", "InterTabular-Medium.ttf", "Inter", "Inter Tabular"),
    ("Unbounded-Bold.ttf", "UnboundedTabular-Bold.ttf", "Unbounded", "Unbounded Tabular"),
]
DIGITS = "0123456789"


def open_font(data):
    return TTFont(io.BytesIO(data), recalcTimestamp=False, lazy=True)


def tnum_map(font):
    """Glyph name to tabular glyph name, from the tnum lookups of the default language systems."""
    gsub = font["GSUB"].table
    features = gsub.FeatureList.FeatureRecord
    lookups = set()
    for script in gsub.ScriptList.ScriptRecord:
        lang = script.Script.DefaultLangSys
        if lang is None:
            continue
        for index in lang.FeatureIndex:
            if features[index].FeatureTag == "tnum":
                lookups.update(features[index].Feature.LookupListIndex)
    mapping = {}
    for index in sorted(lookups):
        for sub in gsub.LookupList.Lookup[index].SubTable:
            if sub.LookupType != 1:
                raise SystemExit("tnum uses a lookup that is not a single substitution")
            mapping.update(sub.mapping)
    return mapping


def rename(font, old, new):
    """The family, full, PostScript and unique names with `new` in place of `old`."""
    old_ps, new_ps = old.replace(" ", ""), new.replace(" ", "")
    for record in font["name"].names:
        text = record.toUnicode()
        if record.nameID in (1, 16, 21):
            text = new + text[len(old):] if text.startswith(old) else text
        elif record.nameID == 4:
            text = new + text[len(old):] if text.startswith(old) else text
        elif record.nameID in (3, 6):
            text = text.replace(old_ps + "-", new_ps + "-")
        else:
            continue
        record.string = text


def build(original, old, new):
    """The derived font's bytes and the characters it remaps."""
    font = open_font(original)
    # Read from a second copy, so the saved font's GSUB is never decompiled and written back.
    mapping = tnum_map(open_font(original))
    remapped = set()
    for table in font["cmap"].tables:
        for code, glyph in list(table.cmap.items()):
            if glyph in mapping:
                table.cmap[code] = mapping[glyph]
                remapped.add(code)
    rename(font, old, new)
    out = io.BytesIO()
    font.save(out)
    return out.getvalue(), sorted(remapped)


def glyph_problems(original, derived, label):
    """What differs between the two fonts apart from the character map and the names."""
    a, b = open_font(original), open_font(derived)
    problems = []
    if a.getGlyphOrder() != b.getGlyphOrder():
        problems.append(f"{label}: the glyph set differs")
        return problems
    for name in a.getGlyphOrder():
        if a["glyf"][name].compile(a["glyf"]) != b["glyf"][name].compile(b["glyf"]):
            problems.append(f"{label}: outline of {name} differs")
        if a["hmtx"][name] != b["hmtx"][name]:
            problems.append(f"{label}: metrics of {name} differ")
    for tag in a.keys():
        if tag in ("cmap", "name", "head", "glyf", "loca", "hmtx", "GlyphOrder"):
            continue
        if a.getTableData(tag) != b.getTableData(tag):
            problems.append(f"{label}: table {tag} differs")
    for field in ("unitsPerEm", "xMin", "yMin", "xMax", "yMax", "indexToLocFormat"):
        if getattr(a["head"], field) != getattr(b["head"], field):
            problems.append(f"{label}: head.{field} differs")
    if (a["head"].created, a["head"].modified) != (b["head"].created, b["head"].modified):
        problems.append(f"{label}: the timestamps differ")
    cmap = b.getBestCmap()
    advances = {b["hmtx"][cmap[ord(ch)]][0] for ch in DIGITS}
    if len(advances) != 1:
        problems.append(f"{label}: 0-9 have different advances {sorted(advances)}")
    return problems


def main():
    check = "--check" in sys.argv[1:]
    problems = []
    for source, target, old, new in FONTS:
        original = (ORIGINALS / source).read_bytes()
        data, remapped = build(original, old, new)
        again, _ = build(original, old, new)
        if data != again:
            problems.append(f"{target}: two builds differ")
        problems += glyph_problems(original, data, target)
        listing = "".join(chr(c) for c in remapped if c < 0x2000)
        if check:
            path = OUT / target
            if not path.is_file() or path.read_bytes() != data:
                problems.append(f"{target}: the file on disk is not what the script writes")
        else:
            OUT.mkdir(parents=True, exist_ok=True)
            (OUT / target).write_bytes(data)
            print(f"{target}: {len(remapped)} characters remapped: {listing!a}")
    for line in problems:
        print(line, file=sys.stderr)
    if check:
        print("the fonts are as the script makes them" if not problems else "check failed")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
