#!/usr/bin/env python3
"""Draw the StatsCraft app icon and write it out.

The icon is a rounded square in the Dracula background colour, with
three ascending bars in the brand accents and a small violet plot line
across them - the statistics in one mark.

It is written as:
  - packaging/assets/icon.svg      the vector source, for the Linux
                                   package and the desktop entry
  - packaging/assets/app.ico       the Windows icon, at 16, 24, 32, 48,
                                   64 and 256 pixels, for the installer
                                   and the taskbar
  - packaging/assets/app-64.png    a PNG for the README and the docs

The rasterising is done by drawing with Pillow, which is a raster
library but draws the shapes directly - the same geometry the SVG has,
so the two agree.

Run:  python tools/build_app_icon.py
"""
import os
import struct
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(REPO, "packaging", "assets")

# The Dracula background and the brand accents.
BG = (40, 42, 54)  # #282A36
BLUE = (47, 111, 237)  # the accent blue
MID = (74, 154, 245)  # the mid blue
TEAL = (15, 168, 126)  # the teal
VIOLET = (189, 147, 249)  # the Dracula purple
FG = (248, 248, 242)  # #F8F8F2


def rounded_mask(size, radius):
    """A Pillow mask of a rounded square."""
    from PIL import Image, ImageDraw

    m = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(m)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return m


def draw_icon(size):
    """The icon at one size, as an RGBA image."""
    from PIL import Image, ImageDraw

    s = size
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    # The tile.
    radius = max(2, int(s * 0.22))
    tile = Image.new("RGBA", (s, s), BG + (255,))
    # A soft violet glow in the top-left, so the tile has some depth.
    glow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    g = int(s * 0.48)
    gd.ellipse([int(-s * 0.12), int(-s * 0.28), g, g], fill=VIOLET + (46,))
    tile = Image.alpha_composite(tile, glow)
    # A thin violet edge.
    edge = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    ImageDraw.Draw(edge).rounded_rectangle(
        [0, 0, s - 1, s - 1], radius=radius, outline=VIOLET + (120,), width=max(1, s // 48)
    )
    tile = Image.alpha_composite(tile, edge)
    img.paste(tile, (0, 0), rounded_mask(s, radius))

    d = ImageDraw.Draw(img)
    # The baseline.
    base = int(s * 0.78)
    # Three ascending bars.
    bw = int(s * 0.155)
    gap = int(s * 0.075)
    left = int(s * 0.19)
    heights = [int(s * 0.26), int(s * 0.42), int(s * 0.60)]
    colors = [BLUE, MID, TEAL]
    for i, (h, c) in enumerate(zip(heights, colors)):
        x0 = left + i * (bw + gap)
        x1 = x0 + bw
        # A rounded top.
        d.rounded_rectangle([x0, base - h, x1 - 1, base], radius=max(1, bw // 4), fill=c + (255,))

    # The plot line across the bars: a rising line with three points,
    # which is what a scatter plot looks like.
    pts = []
    for i, (h, c) in enumerate(zip(heights, colors)):
        x0 = left + i * (bw + gap)
        cx = x0 + bw // 2
        cy = base - h - int(s * 0.10)
        pts.append((cx, cy))
    if len(pts) >= 2:
        d.line(pts, fill=FG + (230,), width=max(1, s // 24), joint="curve")
    for (cx, cy) in pts:
        r = max(1, int(s * 0.035))
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=FG + (255,))

    return img


def write_ico(images):
    """Write a multi-resolution .ico."""
    # The ICO format: a header, a directory of entries, then the
    # images as PNGs (which Windows accepts from Vista on).
    out = os.path.join(ASSETS, "app.ico")
    header = struct.pack("<HHH", 0, 1, len(images))
    entries = b""
    offset = 6 + 16 * len(images)
    blobs = []
    for (size, data) in images:
        w = 0 if size >= 256 else size
        entries += struct.pack("<BBBBHHII", w, w, 0, 0, 1, 32, len(data), offset)
        offset += len(data)
        blobs.append(data)
    with open(out, "wb") as f:
        f.write(header + entries + b"".join(blobs))
    return out


def main():
    try:
        from PIL import Image
    except ImportError:
        print("Pillow is needed: pip install pillow")
        return 1

    os.makedirs(ASSETS, exist_ok=True)

    # The multi-resolution .ico.
    blobs = []
    for size in [16, 24, 32, 48, 64, 128, 256]:
        img = draw_icon(size)
        import io

        buf = io.BytesIO()
        img.save(buf, format="PNG")
        blobs.append((size, buf.getvalue()))
    path = write_ico(blobs)
    print(f"wrote {path} ({os.path.getsize(path)} bytes, {len(blobs)} sizes)")

    # The PNG, for the docs.
    png = draw_icon(256)
    p = os.path.join(ASSETS, "app-256.png")
    png.save(p)
    print(f"wrote {p}")

    # The SVG, for the Linux package.
    svg = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <!-- The StatsCraft app icon: a rounded square in the Dracula
       background, three ascending bars in the brand accents, and a
       rising plot line across them. -->
  <defs>
    <linearGradient id="tile" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#343746"/>
      <stop offset="1" stop-color="#282A36"/>
    </linearGradient>
  </defs>
  <rect x="1" y="1" width="62" height="62" rx="14" fill="url(#tile)"
        stroke="#BD93F9" stroke-opacity="0.55" stroke-width="1.5"/>
  <ellipse cx="18" cy="12" rx="22" ry="18" fill="#BD93F9" fill-opacity="0.16"/>
  <!-- The baseline. -->
  <rect x="11" y="49" width="42" height="3" rx="1.5" fill="#F8F8F2" fill-opacity="0.9"/>
  <!-- Three ascending bars. -->
  <rect x="12" y="30" width="10" height="19" rx="3" fill="#2F6FED"/>
  <rect x="27" y="22" width="10" height="27" rx="3" fill="#4A9AF5"/>
  <rect x="42" y="13" width="10" height="36" rx="3" fill="#0FA87E"/>
  <!-- The plot line. -->
  <path d="M17 24 L32 16 L47 7" stroke="#F8F8F2" stroke-width="2.6"
        stroke-linecap="round" stroke-linejoin="round" fill="none"/>
  <circle cx="17" cy="24" r="2.6" fill="#F8F8F2"/>
  <circle cx="32" cy="16" r="2.6" fill="#F8F8F2"/>
  <circle cx="47" cy="7" r="2.6" fill="#F8F8F2"/>
</svg>
"""
    p = os.path.join(ASSETS, "icon.svg")
    with open(p, "w", encoding="utf-8", newline="\n") as f:
        f.write(svg)
    print(f"wrote {p}")

    # A preview sheet, so the icon can be looked at.
    sheet = Image.new("RGBA", (6 * 140 + 20, 160), (245, 247, 251, 255))
    for i, size in enumerate([16, 24, 32, 48, 64, 128]):
        im = draw_icon(size)
        sheet.alpha_composite(im, (10 + i * 140, 20))
    preview = r"C:/Users/Saurav/AppData/Local/Temp/opencode/app_icon_sizes.png"
    sheet.save(preview)
    print(f"wrote {preview}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
