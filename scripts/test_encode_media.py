import io
import json
import os
import random
import tempfile
import unittest
from contextlib import redirect_stdout

from PIL import Image, ImageDraw

import importlib.util

_spec = importlib.util.spec_from_file_location(
    "encode_media", os.path.join(os.path.dirname(os.path.abspath(__file__)), "encode-media.py"))
encode_media = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(encode_media)


def webp_durations(path):
    with open(path, "rb") as f:
        data = f.read()
    durations, pos = [], 12
    while pos + 8 <= len(data):
        tag, size = data[pos:pos + 4], int.from_bytes(data[pos + 4:pos + 8], "little")
        if tag == b"ANMF":
            durations.append(int.from_bytes(data[pos + 8 + 12:pos + 8 + 15], "little"))
        pos += 8 + size + (size & 1)
    return durations


def make_work(root, budget_kb, noisy=False):
    folder = os.path.join(root, "animations", "demo@1x")
    os.makedirs(folder)
    size = 64 if noisy else 8
    rnd = random.Random(1)
    for i in range(3):
        img = Image.new("RGBA", (size, size), (200, 40, 40, 255))
        if noisy:
            img.putdata([(rnd.randrange(256), rnd.randrange(256), rnd.randrange(256), 255)
                         for _ in range(size * size)])
        img.putpixel((1, 1), (i * 80, 0, 0, 255))
        if i == 0:
            img.putpixel((0, 0), (0, 0, 0, 0))
        img.save(os.path.join(folder, "f%03d.png" % i))
    timeline = {
        "frames": [{"file": "f000.png", "ms": 100}, {"file": "f001.png", "ms": 200},
                   {"file": "f002.png", "ms": 300}],
        "poster": 1, "grounds": ["dark", "light"], "budgetKb": budget_kb}
    with open(os.path.join(folder, "timeline.json"), "w") as f:
        json.dump(timeline, f)
    return folder, timeline


class EncodeTests(unittest.TestCase):
    def test_webp_and_gifs(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder, timeline = make_work(tmp, 600)
            out = os.path.join(tmp, "out")
            os.makedirs(out)
            with redirect_stdout(io.StringIO()):
                encode_media.encode_animation(folder, "demo@1x", timeline, 1, out)

            with Image.open(os.path.join(out, "demo@1x.webp")) as webp:
                self.assertEqual(webp.n_frames, 3)
                self.assertEqual(webp.info["loop"], 0)
                # Pillow does not expose WebP frame durations: read the ANMF chunks.
                self.assertEqual(webp_durations(os.path.join(out, "demo@1x.webp")), [100, 200, 300])
                webp.seek(0)
                self.assertEqual(webp.convert("RGBA").getpixel((0, 0))[3], 0)

            for ground, rgb in (("dark", (11, 11, 11)), ("light", (255, 255, 255))):
                with Image.open(os.path.join(out, "demo@1x-%s.gif" % ground)) as gif:
                    self.assertEqual(gif.n_frames, 3)
                    self.assertEqual(gif.info["loop"], 0)
                    gif.seek(0)
                    self.assertEqual(gif.info["duration"], 100)
                    self.assertEqual(gif.convert("RGB").getpixel((0, 0)), rgb)

            with open(os.path.join(folder, "f001.png"), "rb") as f:
                expected = f.read()
            with open(os.path.join(out, "demo@1x-still.png"), "rb") as f:
                self.assertEqual(f.read(), expected)

    def test_gif_delays_play_as_written(self):
        # A pointer glide of 16/17 ms frames: browsers show a delay of 10 ms or less as 100 ms.
        with tempfile.TemporaryDirectory() as tmp:
            folder = os.path.join(tmp, "animations", "glide@1x")
            os.makedirs(folder)
            ms = [400] + [16, 17, 17] * 12 + [900]
            for i in range(len(ms)):
                img = Image.new("RGBA", (8, 8), (200, 40, 40, 255))
                img.putpixel((i % 8, 3), (0, 0, 255, 255))
                img.save(os.path.join(folder, "f%03d.png" % i))
            timeline = {"frames": [{"file": "f%03d.png" % i, "ms": d} for i, d in enumerate(ms)],
                        "poster": 0, "grounds": ["dark"], "budgetKb": 600}
            out = os.path.join(tmp, "out")
            os.makedirs(out)
            with redirect_stdout(io.StringIO()):
                encode_media.encode_animation(folder, "glide@1x", timeline, 1, out)
            with Image.open(os.path.join(out, "glide@1x-dark.gif")) as gif:
                delays = []
                for i in range(gif.n_frames):
                    gif.seek(i)
                    delays.append(gif.info["duration"])
            self.assertTrue(all(d >= 20 and d % 10 == 0 for d in delays), delays)
            self.assertLessEqual(abs(sum(delays) - sum(ms)), 10, (sum(delays), sum(ms)))
            self.assertEqual(delays[0], 400)
            self.assertEqual(delays[-1] >= 20, True)

    def test_gif_keeps_the_ground_exact_in_a_busy_frame(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder = os.path.join(tmp, "animations", "busy@1x")
            os.makedirs(folder)
            rnd = random.Random(7)
            img = Image.new("RGBA", (48, 48), (0, 0, 0, 0))
            # Anti-aliased greys near the ground colour, and some colour.
            img.putdata([(rnd.randrange(8, 40),) * 3 + (rnd.randrange(30, 255),) if (x // 3) % 2 else (0, 0, 0, 0)
                         for y in range(48) for x in range(48)])
            img.save(os.path.join(folder, "f000.png"))
            timeline = {"frames": [{"file": "f000.png", "ms": 100}], "poster": 0,
                        "grounds": ["dark", "light"], "budgetKb": 600}
            out = os.path.join(tmp, "out")
            os.makedirs(out)
            with redirect_stdout(io.StringIO()):
                encode_media.encode_animation(folder, "busy@1x", timeline, 1, out)
            for ground, rgb in (("dark", (11, 11, 11)), ("light", (255, 255, 255))):
                with Image.open(os.path.join(out, "busy@1x-%s.gif" % ground)) as gif:
                    self.assertEqual(gif.convert("RGB").getpixel((0, 0)), rgb)  # x=0: transparent

    def test_scale_two_makes_no_gif(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder, timeline = make_work(tmp, 600)
            out = os.path.join(tmp, "out")
            os.makedirs(out)
            with redirect_stdout(io.StringIO()):
                encode_media.encode_animation(folder, "demo@2x", timeline, 2, out)
            self.assertTrue(os.path.exists(os.path.join(out, "demo@2x.webp")))
            self.assertFalse(any(n.endswith(".gif") for n in os.listdir(out)))

    def test_over_budget_is_printed(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder, timeline = make_work(tmp, 1, noisy=True)
            out = os.path.join(tmp, "out")
            os.makedirs(out)
            buf = io.StringIO()
            with redirect_stdout(buf):
                encode_media.encode_animation(folder, "demo@1x", timeline, 1, out)
            self.assertIn("OVER BUDGET", buf.getvalue())

    def test_within_budget_is_quiet(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder, timeline = make_work(tmp, 600)
            out = os.path.join(tmp, "out")
            os.makedirs(out)
            buf = io.StringIO()
            with redirect_stdout(buf):
                encode_media.encode_animation(folder, "demo@1x", timeline, 1, out)
            self.assertNotIn("OVER BUDGET", buf.getvalue())

    def test_sheet_refuses_stills_it_cannot_place(self):
        with tempfile.TemporaryDirectory() as tmp:
            for still in ({"path": "stills/x/a.png", "group": "mystery", "dark": True},
                          {"path": "stills/details/other-bars-dark@1x.png", "group": "details", "dark": True}):
                with self.assertRaises(ValueError):
                    encode_media.write_sheet(tmp, {"stills": [still], "animations": []})

    def test_bake_hero(self):
        with tempfile.TemporaryDirectory() as tmp:
            media = os.path.join(tmp, "media")
            out = os.path.join(tmp, "out")
            os.makedirs(os.path.join(media, "plates"))
            os.makedirs(os.path.join(out, "animations"))
            plate = Image.new("RGBA", (30, 4), (10, 20, 30, 255))
            plate.putpixel((30 - 20 + 3, 3), (1, 2, 3, 255))  # bottom row of the rightmost 20 px, x=3
            plate_path = os.path.join(media, "plates", "p.png")
            plate.save(plate_path)
            frames = [Image.new("RGBA", (6, 12), (0, 0, 0, 0)) for _ in range(2)]
            frames[1].paste((255, 255, 255, 255), (0, 0, 6, 6))
            frames[0].save(os.path.join(out, "animations", "h.webp"), save_all=True,
                           append_images=frames[1:], duration=[100, 200], loop=0, lossless=True)
            frames[1].save(os.path.join(out, "animations", "h-still.png"))
            spec = {"out": "docs/images/x", "animation": "animations/h",
                    "colour": "#6f4df6", "width": 20, "radius": 3}
            with redirect_stdout(io.StringIO()):
                encode_media.bake_hero(out, plate_path, spec)
            with Image.open(os.path.join(out, "bake", "x.png")) as png:
                self.assertEqual(png.size, (20, 12))
                self.assertEqual(png.getpixel((10, 0)), (0x6f, 0x4d, 0xf6, 255))
                self.assertEqual(png.getpixel((3, 11)), (1, 2, 3, 255))
                self.assertEqual(png.getpixel((0, 0))[3], 0)
            with Image.open(os.path.join(out, "bake", "x.webp")) as webp:
                self.assertEqual(webp.n_frames, 2)
                self.assertEqual(webp.size, (20, 12))
            self.assertEqual(webp_durations(os.path.join(out, "bake", "x.webp")), [100, 200])

    def test_bake_is_exact_frame_by_frame(self):
        # The README hero's details open and close. A lossy encode left the closed panel's edges
        # behind as a faint box and shifted the field's purple, so every frame must come back
        # exactly: the field is the colour asked for, and closed again means as before.
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "out")
            os.makedirs(os.path.join(out, "animations"))
            plate_path = os.path.join(tmp, "p.png")
            Image.new("RGBA", (200, 8), (28, 28, 28, 255)).save(plate_path)
            closed = Image.new("RGBA", (160, 120), (0, 0, 0, 0))
            opened = closed.copy()
            ImageDraw.Draw(opened).rounded_rectangle((30, 10, 130, 100), 10, fill=(11, 11, 11, 255),
                                                     outline=(38, 38, 38, 255))
            closed.save(os.path.join(out, "animations", "h.webp"), save_all=True,
                        append_images=[opened, closed], duration=[500, 500, 500], loop=0, lossless=True)
            closed.save(os.path.join(out, "animations", "h-still.png"))
            spec = {"out": "docs/images/x", "animation": "animations/h",
                    "colour": "#6f4df6", "width": 200, "radius": 10}
            with redirect_stdout(io.StringIO()):
                encode_media.bake_hero(out, plate_path, spec)
            decoded = []
            with Image.open(os.path.join(out, "bake", "x.webp")) as webp:
                for i in range(webp.n_frames):
                    webp.seek(i)
                    webp.load()
                    decoded.append(webp.convert("RGBA").copy())
            self.assertEqual(decoded[0].tobytes(), decoded[2].tobytes())
            self.assertEqual(decoded[2].getpixel((100, 60)), (0x6f, 0x4d, 0xf6, 255))
            self.assertEqual(decoded[1].getpixel((100, 60)), (11, 11, 11, 255))

    def test_bake_refuses_a_narrow_plate(self):
        with tempfile.TemporaryDirectory() as tmp:
            media = os.path.join(tmp, "media")
            os.makedirs(os.path.join(media, "plates"))
            Image.new("RGBA", (10, 4)).save(os.path.join(media, "plates", "p.png"))
            spec = {"out": "x", "animation": "animations/h",
                    "colour": "#000000", "width": 20, "radius": 3}
            with self.assertRaises(ValueError):
                encode_media.bake_hero(tmp, os.path.join(media, "plates", "p.png"), spec)

    def test_bake_refuses_a_hero_wider_than_the_width(self):
        with tempfile.TemporaryDirectory() as tmp:
            media = os.path.join(tmp, "media")
            os.makedirs(os.path.join(media, "plates"))
            os.makedirs(os.path.join(tmp, "animations"))
            Image.new("RGBA", (30, 4)).save(os.path.join(media, "plates", "p.png"))
            Image.new("RGBA", (25, 12)).save(os.path.join(tmp, "animations", "h-still.png"))
            spec = {"out": "x", "animation": "animations/h",
                    "colour": "#000000", "width": 20, "radius": 3}
            with self.assertRaises(ValueError):
                encode_media.bake_hero(tmp, os.path.join(media, "plates", "p.png"), spec)


if __name__ == "__main__":
    unittest.main()
