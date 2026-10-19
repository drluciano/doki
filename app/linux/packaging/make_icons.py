"""Draws Doki's Linux icons, the same art as the Windows icon.

Run from anywhere with Pillow and the Inter font installed:
    python3 app/linux/packaging/make_icons.py
It rewrites icons/<size>/dev.doki.doki.png next to this script. The Flatpak
installs them in the icon theme; the 64 px one is also the tray icon.
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[1] / "windows" / "installer"))

from make_art import icon_image  # noqa: E402

SIZES = [32, 48, 64, 128, 256, 512]


def main():
    for size in SIZES:
        out = HERE / "icons" / f"{size}x{size}" / "dev.doki.doki.png"
        out.parent.mkdir(parents=True, exist_ok=True)
        icon_image(size).save(out, optimize=True)


if __name__ == "__main__":
    main()
