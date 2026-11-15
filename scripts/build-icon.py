"""Rebuilds src/Sobreiro/Assets/sobreiro.ico from scripts/sobreiro-mark.svg.

The mark is the cork oak's trunk in section on the Night tile (tile #6f4df6, sign #ffffff). Each size is
rendered to a PNG by headless Chrome (or Edge), then the PNGs are packed as PNG-in-ICO entries. Standard
library only.

    python scripts/build-icon.py [--browser PATH]
"""
import argparse
import os
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

SIZES = [16, 24, 32, 48, 256]
ROOT = Path(__file__).resolve().parent.parent
SVG = ROOT / "scripts" / "sobreiro-mark.svg"
ICO = ROOT / "src" / "Sobreiro" / "Assets" / "sobreiro.ico"
BROWSERS = [
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
]


def find_browser(given):
    for candidate in ([given] if given else []) + BROWSERS + [shutil.which("chrome") or "", shutil.which("msedge") or ""]:
        if candidate and os.path.exists(candidate):
            return candidate
    sys.exit("No Chrome or Edge found; pass --browser PATH.")


def render(browser, size, work):
    page = work / f"mark-{size}.html"
    svg = SVG.read_text(encoding="utf-8").strip()
    page.write_text(
        "<!doctype html><meta charset=utf-8><style>html,body{margin:0;background:transparent}"
        f"svg{{display:block;width:{size}px;height:{size}px}}</style>{svg}",
        encoding="utf-8",
    )
    png = work / f"mark-{size}.png"
    subprocess.run(
        [
            browser, "--headless=new", "--disable-gpu", "--hide-scrollbars",
            f"--user-data-dir={work / 'profile'}", "--default-background-color=00000000",
            f"--window-size={size},{size}", f"--screenshot={png}", page.as_uri(),
        ],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=60,
    )
    data = png.read_bytes()
    width, height = struct.unpack(">II", data[16:24])
    if (width, height) != (size, size):
        sys.exit(f"The browser made a {width}x{height} PNG, not {size}x{size}.")
    return data


def pack(pngs):
    header = struct.pack("<HHH", 0, 1, len(pngs))
    offset = 6 + 16 * len(pngs)
    entries, blobs = b"", b""
    for size, data in pngs:
        entries += struct.pack("<BBBBHHII", size % 256, size % 256, 0, 0, 1, 32, len(data), offset)
        blobs += data
        offset += len(data)
    return header + entries + blobs


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--browser")
    browser = find_browser(parser.parse_args().browser)
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        pngs = [(size, render(browser, size, work)) for size in SIZES]
    ICO.parent.mkdir(parents=True, exist_ok=True)
    ICO.write_bytes(pack(pngs))
    print(f"Wrote {ICO} ({ICO.stat().st_size} bytes, sizes {SIZES})")


if __name__ == "__main__":
    main()
