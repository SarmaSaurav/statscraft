#!/usr/bin/env python3
"""Generate the StatsCraft icon set as SVG.

Every icon is a rounded tile in one of the Dracula accents, with the
glyph drawn on top in a near-white. That way an icon carries its own
colour - the ribbon reads as a set of distinct, colourful buttons
rather than a row of grey glyphs - and the near-white glyph stays
legible on a white surface as well as on a dark one, because the
coloured tile behind it gives the contrast.

Every icon is a 24x24 SVG with:
  - a rounded rect tile, 22x22 at an offset of 1, with a radius of 5
  - a soft top highlight so the tile has a little depth
  - the glyph as filled paths or stroked paths in #F8F8F2

The glyphs are the same set as before - the shapes are proven to read
at 16 px - but they are now drawn on a coloured tile instead of
tinted at draw time.

Run:  python tools/build_icons2.py
"""
import os
import re
import subprocess
import sys

OUT = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets", "icons")

# The Dracula accents.
CYAN = "#8BE9FD"
GREEN = "#50FA7B"
ORANGE = "#FFB86C"
PINK = "#FF79C6"
PURPLE = "#BD93F9"
RED = "#FF5555"
YELLOW = "#F1FA8C"
BLUE = "#6272A4"

GLYPH = "#F8F8F2"

# name -> (accent, [glyph path data], [glyph circle/rect shapes])
# The glyph paths are drawn on a 24x24 grid, in the near-white.
ICONS = {
    # ---- file / io ----
    "open": (ORANGE, [
        # folder
        "M3 7.5 A1.5 1.5 0 0 1 4.5 6 H9.5 L11.5 8.5 H19.5 A1.5 1.5 0 0 1 21 10 V18 "
        "A1.5 1.5 0 0 1 19.5 19.5 H4.5 A1.5 1.5 0 0 1 3 18 Z",
        "M7 12.5 H17",
        "M7 15.5 H13",
    ], []),
    "save": (ORANGE, [
        "M4.5 4.5 H16.5 L19.5 7.5 V19.5 H4.5 Z",
        "M8 4.5 V9.5 H16 V4.5",
        "M8 19.5 V14 H16 V19.5",
    ], []),
    "export": (ORANGE, [
        "M4 15 V18.5 A1.5 1.5 0 0 0 5.5 20 H18.5 A1.5 1.5 0 0 0 20 18.5 V15",
        "M12 3.5 V15",
        "M7.5 8.5 L12 3.5 L16.5 8.5",
    ], []),
    "print": (ORANGE, [
        "M7 8 V3.5 H17 V8",
        "M7 17 H4.5 A1.5 1.5 0 0 1 3 15.5 V10 A1.5 1.5 0 0 1 4.5 8.5 H19.5 "
        "A1.5 1.5 0 0 1 21 10 V15.5 A1.5 1.5 0 0 1 19.5 17 H17",
        "M7 14 H17 V20.5 H7 Z",
    ], []),
    # ---- edit / history ----
    "undo": (YELLOW, [
        "M9 6.5 H5.5 V3",
        "M5.8 6.2 A8 8 0 1 1 4.5 12",
    ], []),
    "redo": (YELLOW, [
        "M15 6.5 H18.5 V3",
        "M18.2 6.2 A8 8 0 1 0 19.5 12",
    ], []),
    "cut": (YELLOW, [
        "M6.5 6.5 L17 17.5",
        "M17 6.5 L6.5 17.5",
    ], [
        # the two blades' rings
        ("circle", 6.0, 18.5, 2.4),
        ("circle", 18.0, 18.5, 2.4),
    ]),
    "copy": (YELLOW, [
        "M9 9 H19 V20 H9 Z",
        "M15 9 V4 H5 V15 H9",
    ], []),
    "paste": (YELLOW, [
        "M8 4 H16 V6 H8 Z",
        "M6 6 H18 V20 H6 Z",
    ], []),
    # ---- data ----
    "insert_variable": (PINK, [
        # a column with a label
        "M4 20 H20",
        "M7 20 V11",
        "M12 20 V6",
        "M17 20 V9",
    ], []),
    "insert_case": (PINK, [
        "M4 6 H20 V19 H4 Z",
        "M4 10 H20",
        "M9 6 V19",
        "M15 6 V19",
        "M6 8.5 H8",
    ], []),
    "value_labels": (PINK, [
        "M4 5.5 H20 V18.5 H4 Z",
        "M4 9.5 H20",
        "M7 14 H10",
        "M14 14 H17",
    ], []),
    "variables": (PINK, [
        # the StatsBench variable columns
        "M4 5 H20 V19 H4 Z",
        "M4 9 H20",
        "M4 13 H20",
        "M4 16 H20",
        "M9 5 V19",
        "M15 5 V19",
    ], []),
    "data_view": (PINK, [
        "M4 5.5 H20 V18.5 H4 Z",
        "M4 9.5 H20",
        "M4 14 H20",
        "M9 5.5 V18.5",
        "M15 5.5 V18.5",
    ], []),
    "find": (PINK, [
        "M10.5 16 A5.5 5.5 0 1 1 10.5 5 A5.5 5.5 0 0 1 10.5 16 Z",
        "M14.5 14.5 L20 20",
    ], []),
    "filter": (PINK, [
        "M4 5.5 H20 L14 12.5 V19 L10 21 V12.5 Z",
    ], []),
    "refresh": (PINK, [
        "M20 12 A8 8 0 1 1 17 5.5",
        "M17 3 V6.5 H13.5",
    ], []),
    "settings": (PINK, [
        # A gear: a ring with square teeth, not rays.
        "M12 8.6 A3.4 3.4 0 1 1 12 15.4 A3.4 3.4 0 0 1 12 8.6 Z",
        "M10.4 2.6 H13.6 L14 5.4 H10 Z",
        "M10.4 21.4 H13.6 L14 18.6 H10 Z",
        "M2.6 10.4 V13.6 L5.4 14 V10 Z",
        "M21.4 10.4 V13.6 L18.6 14 V10 Z",
        "M5.2 5.2 L7.4 7.4 L4.9 9.9 L2.7 7.7 Z",
        "M18.8 5.2 L16.6 7.4 L19.1 9.9 L21.3 7.7 Z",
        "M5.2 18.8 L7.4 16.6 L4.9 14.1 L2.7 16.3 Z",
        "M18.8 18.8 L16.6 16.6 L19.1 14.1 L21.3 16.3 Z",
    ], []),
    # ---- procedures: the data ones ----
    "describe": (PURPLE, [
        # A box plot: a box, a median line, and the whiskers.
        "M3 7 H21",
        "M3 17 H21",
        "M7 7 V17",
        "M17 7 V17",
        "M7 12 H17",
    ], []),
    "histogram": (PURPLE, [
        "M4 20 H21",
        "M4 20 V4",
        "M7 20 V13 H10 V20",
        "M11 20 V8 H14 V20",
        "M15 20 V15 H18 V20",
    ], []),
    "crosstabs": (PURPLE, [
        "M4 4.5 H20 V19.5 H4 Z",
        "M11 4.5 V19.5",
        "M4 10 H20",
        "M15 10 V19.5",
    ], []),
    "explore": (PURPLE, [
        "M4 12 H20",
        "M7 12 V18",
        "M17 12 V18",
        "M9 6 L12 12 L15 8",
    ], []),
    "means": (PURPLE, [
        "M4 20 H20",
        "M7 20 V12",
        "M12 20 V7",
        "M17 20 V14",
    ], []),
    # ---- procedures: the modelling ones ----
    "regression": (GREEN, [
        "M4 4 V20 H20",
        "M6 17 L11 11 L14 13 L19 6",
        "M15 6 H19 V10",
    ], []),
    "anova": (GREEN, [
        "M4 4 V20 H20",
        "M6 17 V11",
        "M11 17 V7",
        "M16 17 V13",
    ], []),
    "ttest": (GREEN, [
        "M5 4 V20",
        "M9 4 V20",
        "M5 12 H9",
        "M12.5 8 H20",
        "M12.5 14 H20",
    ], []),
    "correlation": (GREEN, [
        "M4 20 V4 H20",
        "M6.5 17 L11 12 L14 9 L19 5",
        "M6.5 17 A3 3 0 1 1 12.5 17 A3 3 0 1 1 6.5 17 Z",
    ], []),
    "reliability": (GREEN, [
        "M12 3 L14.2 9.2 L21 9.2 L15.8 13.1 L18 19.6 L12 15.6 L6 19.6 "
        "L8.2 13.1 L3 9.2 L9.8 9.2 Z",
    ], []),
    "factor": (GREEN, [
        "M5 20 V8",
        "M12 20 V4",
        "M19 20 V12",
        "M3 20 H21",
    ], []),
    "roc": (GREEN, [
        "M4 20 V4 H20",
        "M5 19 C10 19 13 12 19 5",
        "M5 19 C11 19 17 11 19 5",
    ], []),
    "glm": (GREEN, [
        "M4 4 V20 H20",
        "M7 20 V13",
        "M12 20 V8",
        "M17 20 V15",
        "M6 9 L18 6",
    ], []),
    # ---- procedures: the reporting ones ----
    "chart": (PURPLE, [
        "M4 4 V20 H20",
        "M7 17 V12",
        "M12 17 V7",
        "M17 17 V10",
    ], []),
    "output": (CYAN, [
        "M5 4.5 H19 V19.5 H5 Z",
        "M8 8 H16",
        "M8 11.5 H16",
        "M8 15 H13",
    ], []),
    "syntax": (CYAN, [
        "M8 8 L4 12 L8 16",
        "M16 8 L20 12 L16 16",
        "M13.5 6 L10.5 18",
    ], []),
    "check": (CYAN, [
        "M5 12.5 L10 17.5 L19 6.5",
    ], []),
    # ---- the measure-level icons ----
    "measure_scale": (PINK, [
        # a ruler
        "M3 15.5 L15.5 3 A1.6 1.6 0 0 1 17.8 5.3 L5.3 17.8 A1.6 1.6 0 0 1 3 15.5 Z",
        "M6.4 12.1 L8.4 14.1",
        "M9 9.5 L11 11.5",
        "M11.6 6.9 L13.6 8.9",
    ], []),
    "measure_nominal": (PINK, [
        # a pencil
        "M4 20 L5.2 15.8 L15.5 5.5 A1.8 1.8 0 0 1 18 8 L7.8 18.3 Z",
        "M13.6 7.4 L16.1 9.9",
    ], []),
}


def svg_for(name, accent, paths, shapes):
    """The SVG text of one icon."""
    out = [
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" '
        f'width="24" height="24" fill="none">',
        f'  <!-- {name}: a {accent} tile with a near-white glyph. -->',
        f'  <rect x="1" y="1" width="22" height="22" rx="5.5" fill="{accent}"/>',
        # A soft top highlight, so the tile is not a flat block.
        f'  <rect x="1" y="1" width="22" height="11" rx="5.5" fill="#FFFFFF" '
        'fill-opacity="0.16"/>',
        f'  <rect x="1" y="1" width="22" height="22" rx="5.5" fill="none" '
        f'stroke="#000000" stroke-opacity="0.16" stroke-width="0.75"/>',
    ]
    for shape in shapes:
        if shape[0] == "circle":
            _, cx, cy, r = shape
            out.append(
                f'  <circle cx="{cx}" cy="{cy}" r="{r}" stroke="{GLYPH}" '
                'stroke-width="1.7" fill="none"/>'
            )
    if paths:
        out.append(
            f'  <g stroke="{GLYPH}" stroke-width="1.7" stroke-linecap="round" '
            'stroke-linejoin="round" fill="none">'
        )
        for d in paths:
            out.append(f'    <path d="{d}"/>')
        out.append("  </g>")
    out.append("</svg>")
    return "\n".join(out) + "\n"


def main():
    os.makedirs(OUT, exist_ok=True)
    written = []
    for name, (accent, paths, shapes) in ICONS.items():
        text = svg_for(name, accent, paths, shapes)
        path = os.path.join(OUT, f"{name}.svg")
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
        written.append(name)
    print(f"wrote {len(written)} icons to {OUT}")
    print(" ".join(sorted(written)))


if __name__ == "__main__":
    main()
