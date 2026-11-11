"""Turns what `Sobreiro.exe --media <work>` rendered into the files in media/.

    encode-media.py <work> <out> [picks.json plate.png]

Copies <work>/stills to <out>/stills; per animation writes <out>/animations/<name>.webp,
<name>-<ground>.gif (1x only) and <name>-still.png; writes <out>/index.html (the contact sheet).
With picks.json and the run's plate image, bakes its "bake" entries into <out>/bake (the README hero).
Prints each animation's WebP size and says OVER BUDGET when it is over its budget.
"""
import html
import json
import os
import shutil
import sys

from PIL import Image, ImageChops, ImageDraw

GROUNDS = {"dark": (11, 11, 11, 255), "light": (255, 255, 255, 255)}  # the page's --bg
TASKBAR = {True: "#1C1C1C", False: "#F0F0F0"}


def gif_timeline(durations):
    """Which frames a GIF keeps, and each one's delay in ms.

    GIF delays are centiseconds, and browsers show any delay of 10 ms or less as 100 ms. So every
    GIF delay is a multiple of 10 and at least 20: a run of short frames (the 16/17 ms pointer
    glide) keeps every other frame or so, each kept frame at its place on the timeline. The delay
    is rounded against the GIF's own clock, so the rounding is carried and the total stays within
    10 ms of the timeline's.
    """
    starts, t = [], 0
    for d in durations:
        starts.append(t)
        t += d
    total = t

    def r10(ms):
        return int((ms + 5) // 10) * 10

    kept, delays, clock, k = [0], [], 0, 0
    while True:
        j = k + 1
        while j < len(durations) and r10(starts[j] - clock) < 20:
            j += 1
        if j >= len(durations):
            delays.append(max(20, r10(total - clock)))
            break
        delay = r10(starts[j] - clock)
        delays.append(delay)
        clock += delay
        kept.append(j)
        k = j
    return kept, delays


def quantize_pinned(rgb, ground):
    """256-colour image whose palette holds `ground` exactly, so the ground never shifts."""
    pal = rgb.quantize(255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    colours = pal.getpalette()[:3 * 255]
    colours += [0] * (3 * 256 - len(colours))
    if tuple(ground) not in {tuple(colours[i:i + 3]) for i in range(0, 768, 3)}:
        nearest = min(range(255), key=lambda i: sum((colours[3 * i + c] - ground[c]) ** 2 for c in range(3)))
        colours[3 * nearest:3 * nearest + 3] = list(ground)
    palette = Image.new("P", (1, 1))
    palette.putpalette(colours)
    out = rgb.quantize(palette=palette, dither=Image.Dither.NONE)
    # Pillow matches palettes through a coarse cache: pin the exact ground pixels by hand.
    index = [colours[i:i + 3] for i in range(0, 768, 3)].index(list(ground))
    diff = ImageChops.difference(rgb, Image.new("RGB", rgb.size, tuple(ground)))
    is_ground = ImageChops.multiply(*[b.point(lambda v: 255 if v == 0 else 0) for b in diff.split()[:2]])
    is_ground = ImageChops.multiply(is_ground, diff.split()[2].point(lambda v: 255 if v == 0 else 0))
    out.paste(index, mask=is_ground)
    return out


def encode_animation(folder, name, timeline, scale, out_dir):
    """Writes the WebP, the GIFs (1x only) and the poster PNG of one animation into out_dir."""
    frames = timeline["frames"]
    cache = {}

    def load(file):
        if file not in cache:
            with Image.open(os.path.join(folder, file)) as img:
                cache[file] = img.convert("RGBA")
        return cache[file]

    images = [load(f["file"]) for f in frames]
    durations = [f["ms"] for f in frames]

    webp_path = os.path.join(out_dir, name + ".webp")
    images[0].save(webp_path, save_all=True, append_images=images[1:], duration=durations,
                   loop=0, lossless=True, quality=100, method=6, exact=True)

    if scale == 1:
        kept, gif_durations = gif_timeline(durations)
        for ground in timeline["grounds"]:
            bg = Image.new("RGBA", images[0].size, GROUNDS[ground])
            flat = [quantize_pinned(Image.alpha_composite(bg, images[i]).convert("RGB"), GROUNDS[ground][:3])
                    for i in kept]
            flat[0].save(os.path.join(out_dir, "%s-%s.gif" % (name, ground)), save_all=True,
                         append_images=flat[1:], duration=gif_durations, loop=0, disposal=1)

    shutil.copyfile(os.path.join(folder, frames[timeline["poster"]]["file"]),
                    os.path.join(out_dir, name + "-still.png"))

    kb = os.path.getsize(webp_path) / 1024
    budget = timeline["budgetKb"]
    line = "%-34s %7.0f KB (budget %d KB)" % (name, kb, budget)
    if kb > budget:
        line += "  OVER BUDGET"
    print(line)


def size_of(path):
    with Image.open(path) as img:
        return img.size


def services_of(path):
    first = os.path.basename(path).split("-")[0]
    return first if first in ("claude", "copilot", "both") else "other"


def item(rel, w, h, taskbar_dark=None):
    """One picture shown on both grounds, with its file name under it."""
    name = html.escape(os.path.basename(rel))
    src = html.escape(rel)
    panels = []
    for ground in ("dark", "light"):
        img = '<img src="%s" width="%d" height="%d" alt="%s">' % (src, w, h, name)
        if taskbar_dark is not None:
            img = '<div class="bar" style="background:%s">%s</div>' % (TASKBAR[taskbar_dark], img)
        panels.append('<div class="ground %s">%s</div>' % (ground, img))
    return '<figure><div class="pair">%s</div><figcaption>%s</figcaption></figure>' % (
        "".join(panels), name)


def still_item(out_dir, rel, taskbar_dark=None):
    w, h = size_of(os.path.join(out_dir, rel))
    if "@2x" in rel:
        w, h = w // 2, h // 2
    return item(rel, w, h, taskbar_dark)


def check_groups(manifest):
    """The sheet must show every still; refuse what it cannot place."""
    for s in manifest["stills"]:
        if s["group"] not in ("details", "pill", "icons", "taskbar"):
            raise ValueError("Cannot place %s on the sheet: unknown group '%s'." % (s["path"], s["group"]))
        if s["group"] == "details" and services_of(s["path"]) == "other":
            raise ValueError("Cannot place %s on the sheet: its services are not claude, copilot or both."
                             % s["path"])


def write_sheet(out_dir, manifest):
    check_groups(manifest)
    stills = sorted(manifest["stills"], key=lambda s: s["path"])
    sections = []

    def section(title, body):
        if body:
            sections.append("<section><h2>%s</h2>%s</section>" % (title, "".join(body)))

    details = [s for s in stills if s["group"] == "details"]
    parts = []
    for services, label in (("claude", "Claude only"), ("copilot", "Copilot only"), ("both", "Claude and Copilot")):
        group = [still_item(out_dir, s["path"]) for s in details if services_of(s["path"]) == services]
        if group:
            parts.append("<h3>%s</h3><div class=\"row\">%s</div>" % (label, "".join(group)))
    section("Details", parts)

    for key, title in (("pill", "Pill"), ("icons", "Icons")):
        group = [still_item(out_dir, s["path"], s["dark"]) for s in stills if s["group"] == key]
        section(title, ['<div class="row">%s</div>' % "".join(group)] if group else [])

    taskbar = [still_item(out_dir, s["path"]) for s in stills if s["group"] == "taskbar"]
    section("Taskbar", ['<div class="row">%s</div>' % "".join(taskbar)] if taskbar else [])

    anims = []
    for a in sorted(manifest["animations"], key=lambda a: a["name"]):
        rel = "animations/%s.webp" % a["name"]
        anims.append(item(rel, a["width"] // a["scale"], a["height"] // a["scale"]))
    section("Animations", ['<div class="row">%s</div>' % "".join(anims)] if anims else [])

    page = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Sobreiro pictures</title>
<style>
body { margin: 0; padding: 24px; background: #fff; color: #111; font-family: system-ui, sans-serif; }
h1, h2, h3 { font-weight: 600; }
.row { display: flex; flex-wrap: wrap; gap: 24px; align-items: flex-start; }
figure { margin: 0; }
figcaption { font-size: 12px; margin-top: 4px; word-break: break-all; max-width: 100%%; }
.pair { display: flex; }
.ground { padding: 16px; }
.ground.dark { background: #0b0b0b; }
.ground.light { background: #ffffff; outline: 1px solid #ddd; outline-offset: -1px; }
.bar { display: inline-block; padding: 8px; line-height: 0; }
img { display: block; }
</style>
</head>
<body>
<h1>Sobreiro pictures</h1>
<p>Every picture at its real size, on a dark and a light page. Pills and icons sit on their taskbar colour.</p>
%s
</body>
</html>
""" % "\n".join(sections)
    with open(os.path.join(out_dir, "index.html"), "w", encoding="utf-8") as f:
        f.write(page)


def bake_hero(out, plate_path, spec):
    """Bakes one animation onto its field for the README, where no CSS can reach.

    The field is spec["colour"], spec["width"] wide and as tall as the animation; the foot is
    the real taskbar plate (its rightmost `width` pixels, bottom-aligned); the frames sit on
    top at the bottom-right; the corners are rounded by spec["radius"].
    Writes <out>/bake/<name>.webp (animated, frame times kept) and <name>.png (the poster).
    """
    name = os.path.basename(spec["out"])
    width, radius = spec["width"], spec["radius"]
    hexc = spec["colour"].lstrip("#")
    field = tuple(int(hexc[i:i + 2], 16) for i in (0, 2, 4)) + (255,)
    anim = os.path.join(out, "animations", os.path.basename(spec["animation"]))
    with Image.open(plate_path) as p:
        plate = p.convert("RGBA")
    if plate.width < width:
        raise ValueError("The plate %s is %d px wide, narrower than the hero's %d px."
                         % (os.path.basename(plate_path), plate.width, width))
    strip = plate.crop((plate.width - width, 0, plate.width, plate.height))

    with Image.open(anim + "-still.png") as s:
        still = s.convert("RGBA")
    if still.width > width:
        raise ValueError("The hero is %d px wide, wider than the bake's %d px." % (still.width, width))
    w, h = width, still.height
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, w - 1, h - 1), radius, fill=255)

    def compose(frame):
        c = Image.new("RGBA", (w, h), field)
        c.alpha_composite(strip, (w - strip.width, h - strip.height))
        c.alpha_composite(frame.convert("RGBA"), (w - frame.width, h - frame.height))
        c.putalpha(mask)
        return c

    frames, durs = [], []
    with Image.open(anim + ".webp") as im:
        for i in range(getattr(im, "n_frames", 1)):
            im.seek(i)
            im.load()
            durs.append(im.info.get("duration", 17))
            frames.append(compose(im.copy()))
    bake_dir = os.path.join(out, "bake")
    os.makedirs(bake_dir, exist_ok=True)
    webp = os.path.join(bake_dir, name + ".webp")
    # Lossless: a lossy encode kept the closed details' edges as a faint box on the field and
    # shifted its purple; the flat field also makes lossless the smaller file (about 50 KB).
    frames[0].save(webp, save_all=True, append_images=frames[1:], duration=durs, loop=0,
                   lossless=True, quality=100, method=6)
    compose(still).save(os.path.join(bake_dir, name + ".png"), optimize=True)
    print("%-34s %7.0f KB; %dx%d (baked)" % (name + ".webp", os.path.getsize(webp) / 1024, w, h))


def main(work, out, picks=None, plate=None):
    with open(os.path.join(work, "manifest.json"), encoding="utf-8") as f:
        manifest = json.load(f)

    shutil.copytree(os.path.join(work, "stills"), os.path.join(out, "stills"))
    anim_out = os.path.join(out, "animations")
    os.makedirs(anim_out)
    for a in manifest["animations"]:
        folder = os.path.join(work, a["folder"])
        with open(os.path.join(folder, "timeline.json"), encoding="utf-8") as f:
            timeline = json.load(f)
        encode_animation(folder, a["name"], timeline, a["scale"], anim_out)
    write_sheet(out, manifest)
    if picks and plate and os.path.exists(picks):
        with open(picks, encoding="utf-8") as f:
            spec = json.load(f)
        for b in spec.get("bake", []):
            bake_hero(out, plate, b)


if __name__ == "__main__":
    if len(sys.argv) not in (3, 5):
        print("usage: encode-media.py <work> <out> [picks.json plate.png]", file=sys.stderr)
        sys.exit(2)
    main(*sys.argv[1:])
