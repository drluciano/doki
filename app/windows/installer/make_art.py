"""Draws Doki's Windows icon and the installer's wizard images.

Run from anywhere with Pillow installed:  python3 app/windows/installer/make_art.py
It rewrites app/windows/runner/resources/app_icon.ico and the BMPs next to this
script. Colors are the light palette in app/lib/theme.dart; the wordmark
matches the splash screen (heavy lavender "doki" over cream).
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
ICON = HERE.parent / "runner" / "resources" / "app_icon.ico"
FONT = "/usr/share/fonts/opentype/inter/Inter-ExtraBold.otf"
FONT_TEXT = "/usr/share/fonts/opentype/inter/Inter-Medium.otf"

CREAM = (250, 247, 242)
LAVENDER = (115, 84, 163)
LAVENDER_SOFT = (237, 230, 247)
MINT = (216, 241, 232)
MUTED = (101, 93, 112)
PINK = (244, 168, 196)

SS = 4  # draw large, then shrink, for smooth edges


def heart(draw, cx, cy, r, fill):
    """A small round heart centered on (cx, cy), r is half its width."""
    lobe = r * 0.55
    draw.ellipse([cx - r, cy - r * 0.75, cx - r + 2 * lobe, cy - r * 0.75 + 2 * lobe], fill=fill)
    draw.ellipse([cx + r - 2 * lobe, cy - r * 0.75, cx + r, cy - r * 0.75 + 2 * lobe], fill=fill)
    draw.polygon([(cx - r * 0.97, cy - r * 0.05), (cx + r * 0.97, cy - r * 0.05), (cx, cy + r * 0.95)], fill=fill)


def icon_image(size):
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    pad = round(s * 0.04)
    d.rounded_rectangle([pad, pad, s - pad, s - pad], radius=round(s * 0.26), fill=LAVENDER)
    font = ImageFont.truetype(FONT, round(s * 0.74))
    # Center the "d" by its ink, not its advance box.
    l, t, r, b = d.textbbox((0, 0), "d", font=font)
    x = (s - (r - l)) / 2 - l - s * 0.03
    y = (s - (b - t)) / 2 - t + s * 0.02
    d.text((x, y), "d", font=font, fill=CREAM)
    if size >= 24:
        heart(d, s * 0.27, s * 0.26, s * 0.085, PINK)
    return img.resize((size, size), Image.LANCZOS)


def write_icon():
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    images = [icon_image(n) for n in sizes]
    images[-1].save(ICON, format="ICO", sizes=[(n, n) for n in sizes], append_images=images[:-1])


def wizard_image(w, h):
    """The tall panel on the first and last pages."""
    s = SS
    img = Image.new("RGB", (w * s, h * s), CREAM)
    d = ImageDraw.Draw(img)
    # Soft shapes, like the app's cards, drifting off the edges.
    d.ellipse([-w * 0.45 * s, h * 0.62 * s, w * 0.75 * s, h * 1.25 * s], fill=LAVENDER_SOFT)
    d.ellipse([w * 0.45 * s, -h * 0.12 * s, w * 1.35 * s, h * 0.32 * s], fill=MINT)
    font = ImageFont.truetype(FONT, round(w * 0.30 * s))
    l, t, r, b = d.textbbox((0, 0), "doki", font=font)
    x = (w * s - (r - l)) / 2 - l
    y = h * 0.40 * s - t
    d.text((x, y), "doki", font=font, fill=LAVENDER)
    small = ImageFont.truetype(FONT_TEXT, round(w * 0.074 * s))
    for i, line in enumerate(["Your saves,", "on your terms"]):
        l2, t2, r2, _ = d.textbbox((0, 0), line, font=small)
        d.text(((w * s - (r2 - l2)) / 2 - l2, y + (b - t) + (h * 0.07 + i * w * 0.105) * s - t2), line, font=small, fill=MUTED)
    heart(d, x + (r - l) + w * 0.06 * s, y + w * 0.03 * s, w * 0.042 * s, PINK)
    return img.resize((w, h), Image.LANCZOS)


def small_image(n):
    """The square in the corner of the inner pages."""
    return icon_image(n).convert("RGBA")


def flatten(img, bg=(255, 255, 255)):
    base = Image.new("RGB", img.size, bg)
    base.paste(img, mask=img.split()[-1])
    return base


def main():
    write_icon()
    # Inno Setup picks the closest size for the screen's scaling.
    for scale in (1, 2):
        wizard_image(164 * scale, 314 * scale).save(HERE / f"wizard-{scale}x.bmp")
        flatten(small_image(55 * scale)).save(HERE / f"wizard-small-{scale}x.bmp")


if __name__ == "__main__":
    main()
