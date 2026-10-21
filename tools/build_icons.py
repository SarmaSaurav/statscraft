#!/usr/bin/env python3
"""Generate the StatsCraft icon set as SVG via the VectorCraft CLI.

Each icon is a 24x24 stroked-path glyph in a single colour. Stroking (not
filling) is what makes an icon read as a modern line icon rather than a
dull 1990s pictogram, so every glyph is built from open paths with a round
cap and a round join.

VectorCraft is driven headlessly: one `file.new` per icon, `path.create` for
each sub-path (SVG path data), then paint/stroke, then export. The exported
SVG is re-read and checked (size, stroke present, no fill) before it is
accepted.
"""
import json
import os
import re
import subprocess
import sys

VC = r"C:/Program Files/VectorCraft/vectorcraft-cli.exe"
OUT = r"D:/The_SPSS_Garage/StatsCraft/assets/icons"

# Every icon is drawn on a 24x24 grid, stroke 1.8, round caps/joins.
# Colour is white: the app tints the icon at draw time, so one asset serves
# every context (normal / hover / disabled).
STROKE_W = 1.8

# name -> list of SVG path-data strings (each is one sub-path)
ICONS = {
    # ---- file / io ----
    "open": [
        # folder: back flap, body, and a document peeking out
        "M3 7.5 A1.5 1.5 0 0 1 4.5 6 H9.5 L11.5 8.5 H19.5 A1.5 1.5 0 0 1 21 10 V18 A1.5 1.5 0 0 1 19.5 19.5 H4.5 A1.5 1.5 0 0 1 3 18 Z",
        "M7 12.5 H17",
        "M7 15.5 H13",
    ],
    "save": [
        # floppy: body, shutter, label
        "M4.5 4.5 H16.5 L19.5 7.5 V19.5 H4.5 Z",
        "M8 4.5 V9.5 H16 V4.5",
        "M8 19.5 V14 H16 V19.5",
    ],
    "export": [
        # tray with an up arrow
        "M4 15 V18.5 A1.5 1.5 0 0 0 5.5 20 H18.5 A1.5 1.5 0 0 0 20 18.5 V15",
        "M12 3.5 V15",
        "M7.5 8.5 L12 3.5 L16.5 8.5",
    ],
    "print": [
        "M7 8 V3.5 H17 V8",
        "M7 17 H4.5 A1.5 1.5 0 0 1 3 15.5 V10 A1.5 1.5 0 0 1 4.5 8.5 H19.5 A1.5 1.5 0 0 1 21 10 V15.5 A1.5 1.5 0 0 1 19.5 17 H17",
        "M7 14 H17 V20.5 H7 Z",
    ],
    # ---- edit / history ----
    "undo": [
        "M9 6.5 H5.5 V3",
        "M5.8 6.2 A8 8 0 1 1 4.5 12",
    ],
    "redo": [
        # single right-curving arrow (mirror of undo), NOT a full cycle
        "M7.5 7.5 H17 A4.5 4.5 0 0 1 17 16.5",
        "M14.5 4 L17.5 7.5 L14.5 11",
    ],
    "cut": [
        # scissors
        "M6.5 6.5 L17 17",
        "M17 6.5 L6.5 17",
        "M6.5 4.5 A2.5 2.5 0 1 0 6.5 9.5 A2.5 2.5 0 1 0 6.5 4.5",
        "M17.5 14.5 A2.5 2.5 0 1 0 17.5 19.5 A2.5 2.5 0 1 0 17.5 14.5",
    ],
    "copy": [
        "M8.5 8.5 H19 V20 H8.5 Z",
        "M15.5 8.5 V5 A1.5 1.5 0 0 0 14 3.5 H5 A1.5 1.5 0 0 0 3.5 5 V14 A1.5 1.5 0 0 0 5 15.5 H8.5",
    ],
    "paste": [
        "M9 4.5 H15 V3 H9 Z",
        "M7 4.5 H6 A1.5 1.5 0 0 0 4.5 6 V20.5 A1.5 1.5 0 0 0 6 22 H18 A1.5 1.5 0 0 0 19.5 20.5 V6 A1.5 1.5 0 0 0 18 4.5 H17",
    ],
    # ---- data model ----
    "insert_variable": [
        # two columns with one highlighted between
        "M3.5 5 H8.5 V19 H3.5 Z",
        "M15.5 5 H20.5 V19 H15.5 Z",
        "M11 5 H13 V19 H11",
    ],
    "insert_case": [
        "M3.5 4.5 H20.5 V9.5 H3.5 Z",
        "M3.5 14.5 H20.5 V19.5 H3.5 Z",
        "M10.5 11 H13.5 V13 H10.5 Z",
    ],
    "value_labels": [
        # tag
        "M3.5 8.5 H12 L19.5 16 L12 20.5 H3.5 Z",
        "M7.5 13 A1 1 0 1 0 7.5 15 A1 1 0 1 0 7.5 13",
    ],
    "find": [
        "M10.5 4 A6.5 6.5 0 1 0 10.5 17 A6.5 6.5 0 1 0 10.5 4",
        "M15.4 15.4 L20 20",
    ],
    "filter": [
        "M3.5 5 H20.5 L14 12.5 V19.5 L10 21.5 V12.5 Z",
    ],
    # ---- analysis ----
    "chart": [
        # bar chart with axes
        "M4 3.5 V20.5 H20.5",
        "M8 20.5 V14",
        "M12.5 20.5 V9.5",
        "M17 20.5 V5.5",
    ],
    "histogram": [
        "M4 3.5 V20.5 H20.5",
        "M7 20.5 V16 H10 V20.5",
        "M10.5 20.5 V11 H13.5 V20.5",
        "M14 20.5 V14 H17 V20.5",
        "M17.5 20.5 V8 H20 V20.5",
    ],
    "regression": [
        "M4 3.5 V20.5 H20.5",
        "M6.5 17.5 L11 12 L14 14 L19.5 6.5",
    ],
    "crosstabs": [
        # 2x2 contingency grid: outer box, one divider each way
        "M4 4.5 H20 V19.5 H4 Z",
        "M12 4.5 V19.5",
        "M4 12 H20",
    ],
    "describe": [
        # box-and-whisker: whisker, box, median line
        "M4 12 H20",
        "M7.5 6.5 H16.5 V17.5 H7.5 Z",
        "M12 6.5 V17.5",
    ],
    "correlation": [
        # scatter plot: axes plus scattered points
        "M4 3.5 V20.5 H20.5",
        "M7.5 16.5 A1.3 1.3 0 1 0 7.5 19.1 A1.3 1.3 0 1 0 7.5 16.5",
        "M11 13 A1.3 1.3 0 1 0 11 15.6 A1.3 1.3 0 1 0 11 13",
        "M14 15 A1.3 1.3 0 1 0 14 17.6 A1.3 1.3 0 1 0 14 15",
        "M16.5 9 A1.3 1.3 0 1 0 16.5 11.6 A1.3 1.3 0 1 0 16.5 9",
        "M12 7.5 A1.3 1.3 0 1 0 12 10.1 A1.3 1.3 0 1 0 12 7.5",
    ],
    "anova": [
        "M4 3.5 V20.5 H20.5",
        "M7 20.5 V12 H10.5 V20.5",
        "M12 20.5 V7 H15.5 V20.5",
        "M17 20.5 V15 H20 V20.5",
    ],
    "ttest": [
        # two paired groups: two bars with a bracket spanning them
        "M4 3.5 V20.5 H20.5",
        "M7 20.5 V12 H10.5 V20.5",
        "M13.5 20.5 V12 H17 V20.5",
        "M7 8.5 H17",
        "M7 6 V11 M17 6 V11",
    ],
    # ---- app chrome ----
    "output": [
        # document with a chart
        "M6 3.5 H14 L18.5 8 V20.5 H6 Z",
        "M14 3.5 V8 H18.5",
        "M9 16.5 V13",
        "M12 16.5 V10.5",
        "M15 16.5 V12.5",
    ],
    "syntax": [
        # angle brackets + underscore
        "M8.5 7 L4 12 L8.5 17",
        "M15.5 7 L20 12 L15.5 17",
        "M10.5 18.5 H14",
    ],
    "variables": [
        # spreadsheet: outer box with a header band and column rules
        "M3.5 4.5 H20.5 V19.5 H3.5 Z",
        "M3.5 8.5 H20.5",
        "M8.5 8.5 V19.5",
        "M14 8.5 V19.5",
    ],
    "refresh": [
        "M20 12 A8 8 0 1 1 17 5.5",
        "M20.5 3.5 V7 H17",
    ],
    "settings": [
        "M12 8.5 A3.5 3.5 0 1 0 12 15.5 A3.5 3.5 0 1 0 12 8.5",
        "M12 2.5 V5 M12 19 V21.5 M2.5 12 H5 M19 12 H21.5 M5.2 5.2 L7 7 M17 17 L18.8 18.8 M18.8 5.2 L17 7 M7 17 L5.2 18.8",
    ],
    "check": [
        "M4.5 12.5 L9.5 17.5 L19.5 6.5",
    ],
}


def run_cli(args):
    r = subprocess.run([VC] + args, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"vectorcraft-cli failed: {r.stderr.strip()[:400]}")
    return r.stdout


def build_icon(name, paths):
    """Build one icon as its own 24x24 VectorCraft document and export SVG."""
    args = ["run"]
    args += ["--cmd", "file.new", "--params", json.dumps({
        "name": name, "width": 24, "height": 24, "units": "Pixels",
        "backgroundContents": "transparent",
    })]
    for d in paths:
        args += ["--cmd", "path.create", "--params", json.dumps({"d": d, "closed": False})]
    # fill none so the stroke reads as a line icon
    args += ["--cmd", "paint.setFill", "--params", json.dumps({"none": True})]
    args += ["--cmd", "paint.setStroke", "--params", json.dumps({"color": "#FFFFFF"})]
    args += ["--cmd", "stroke.set", "--params", json.dumps({
        "weight": STROKE_W, "cap": "round", "join": "round",
    })]
    dest = f"{OUT}/{name}.svg"
    args += ["--export", dest]
    run_cli(args)
    return dest


def verify(name, path):
    """Re-read the exported SVG and check it is a sane 24x24 stroked glyph."""
    svg = open(path, encoding="utf-8").read()
    problems = []
    if 'width="24"' not in svg or 'height="24"' not in svg:
        problems.append("not 24x24")
    if "stroke=" not in svg:
        problems.append("no stroke")
    if 'fill="none"' not in svg:
        problems.append("fill is not none")
    if 'stroke-width="1.8"' not in svg:
        problems.append("wrong stroke width")
    npaths = svg.count("<path")
    if npaths != len(ICONS[name]):
        problems.append(f"{npaths} paths, expected {len(ICONS[name])}")
    return problems


def main():
    os.makedirs(OUT, exist_ok=True)
    only = sys.argv[1:] or list(ICONS)
    bad = 0
    for name in only:
        dest = build_icon(name, ICONS[name])
        problems = verify(name, dest)
        size = os.path.getsize(dest)
        if problems:
            bad += 1
            print(f"  FAIL {name:18} {', '.join(problems)}")
        else:
            print(f"  ok   {name:18} {size:>5}B")
    print(f"\n{len(only) - bad}/{len(only)} icons built into {OUT}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
