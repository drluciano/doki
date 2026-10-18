#!/usr/bin/env python3
"""Check and render the wiki sources in docs/wiki.

Pages link to each other by file name (`[Pairing](Pairing-Devices.md)`) so the
links work when browsing the repository. GitHub wikis address pages without the
extension, so `render` strips `.md` from those links when copying the pages.

    python3 tools/wiki.py check docs/wiki        # every page link resolves
    python3 tools/wiki.py render docs/wiki OUT   # copy pages into OUT for the wiki
"""

import re
import sys
from pathlib import Path

# A Markdown link whose target is a bare page file, optionally with an anchor.
PAGE_LINK = re.compile(r"\]\(([A-Za-z0-9_-]+)\.md(#[^)\s]*)?\)")


def pages(src: Path) -> list[Path]:
    return sorted(src.glob("*.md"))


def check(src: Path) -> int:
    names = {p.stem for p in pages(src)}
    errors = []
    for page in pages(src):
        for n, line in enumerate(page.read_text(encoding="utf-8").splitlines(), 1):
            for m in PAGE_LINK.finditer(line):
                if m.group(1) not in names:
                    errors.append(f"{page}:{n}: link to missing page {m.group(1)}.md")
    if "Home" not in names:
        errors.append(f"{src}: Home.md is required as the wiki's front page")
    for e in errors:
        print(e)
    print(f"{len(names)} pages, {len(errors)} problems")
    return 1 if errors else 0


def render(src: Path, out: Path) -> int:
    out.mkdir(parents=True, exist_ok=True)
    for page in pages(src):
        text = PAGE_LINK.sub(lambda m: f"]({m.group(1)}{m.group(2) or ''})", page.read_text(encoding="utf-8"))
        (out / page.name).write_text(text, encoding="utf-8")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) == 3 and argv[1] == "check":
        return check(Path(argv[2]))
    if len(argv) == 4 and argv[1] == "render":
        return render(Path(argv[2]), Path(argv[3]))
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
