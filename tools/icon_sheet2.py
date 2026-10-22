#!/usr/bin/env python3
"""Render the icon set into a contact sheet, on a light and a dark
background, so the icons can be looked at.

The sheet is an HTML page with the SVGs in a grid, and it is captured
with headless Edge. An SVG on its own is not captured, but an SVG
inside an HTML page is.

Run:  python tools/icon_sheet2.py
"""
import os
import subprocess

D = r"C:/Users/Saurav/AppData/Local/Temp/opencode"
ICONS = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets", "icons")
EDGE = "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"

ORDER = [
    "open", "save", "export", "print", "undo", "redo", "cut", "copy", "paste",
    "insert_variable", "insert_case", "value_labels", "find", "filter", "refresh",
    "settings", "chart", "histogram", "crosstabs", "describe", "regression",
    "anova", "ttest", "correlation", "reliability", "factor", "roc", "glm",
    "means", "explore", "output", "syntax", "variables", "data_view", "check",
    "measure_scale", "measure_nominal",
]


def sheet(bg, out, cols=10):
    names = [n for n in ORDER if os.path.exists(os.path.join(ICONS, n + ".svg"))]
    cells = "".join(
        f'<div class="c"><img src="file:///{ICONS}/{n}.svg"><span>{n}</span></div>'
        for n in names
    )
    label = "#777" if bg == "#FFFFFF" else "#BBB"
    html = f"""<!DOCTYPE html><html><head><meta charset="utf-8"><style>
body {{ margin:0; background:{bg}; font-family:'Segoe UI',sans-serif; }}
.g {{ display:grid; grid-template-columns:repeat({cols},1fr); gap:4px; padding:10px; }}
.c {{ text-align:center; }}
.c img {{ width:50px; height:50px; display:block; margin:0 auto 2px; }}
.c span {{ font-size:9.5px; color:{label}; }}
</style></head><body><div class="g">{cells}</div></body></html>"""
    hp = os.path.join(D, out + ".html")
    with open(hp, "w", encoding="utf-8") as f:
        f.write(html)
    win = os.path.join(D, out + ".png")
    subprocess.run(
        [
            EDGE,
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            f"--screenshot={win}",
            "--window-size=1000,520",
            "--hide-scrollbars",
            "file:///" + hp.replace("\\", "/"),
        ],
        capture_output=True,
    )
    print(out, os.path.exists(win), os.path.getsize(win) if os.path.exists(win) else 0)


if __name__ == "__main__":
    sheet("#FFFFFF", "sheet_light")
    sheet("#282A36", "sheet_dark")
