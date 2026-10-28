from __future__ import annotations

import html.parser
import posixpath
import sys
from pathlib import Path
from urllib.parse import unquote, urljoin, urlsplit


class Page(html.parser.HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.links: list[str] = []
        self.canonical: str | None = None
        self.code_depth = 0

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag in ("code", "pre"):
            self.code_depth += 1
        values = dict(attrs)
        for attr in ("id", "name"):
            if values.get(attr):
                self.ids.add(values[attr])
        if tag == "a" and values.get("href") and not self.code_depth:
            self.links.append(values["href"])
        if tag == "link" and "canonical" in (values.get("rel") or "").split():
            self.canonical = values.get("href")

    def handle_endtag(self, tag: str) -> None:
        if tag in ("code", "pre"):
            self.code_depth = max(0, self.code_depth - 1)


def check(pages: dict[str, str]) -> list[str]:
    parsed: dict[str, Page] = {}
    for path, source in pages.items():
        page = Page()
        page.feed(source)
        parsed[path] = page

    index_canonical = parsed.get("index.html", Page()).canonical
    base_path = urlsplit(index_canonical).path.rstrip("/") if index_canonical else ""
    errors: list[str] = []
    for page_path, page in parsed.items():
        page_url = "/" + page_path.removesuffix("index.html")
        for href in page.links:
            if href.startswith("//") or urlsplit(href).scheme:
                continue
            target_url = urlsplit(urljoin("https://local" + page_url, href))
            target_path = unquote(target_url.path).lstrip("/")
            if base_path and target_path.startswith(base_path.lstrip("/") + "/"):
                target_path = target_path[len(base_path.lstrip("/")) + 1 :]
            elif base_path and target_path == base_path.lstrip("/"):
                target_path = ""
            if not target_path:
                target_path = "index.html"
            elif target_path.endswith("/"):
                target_path += "index.html"
            target_path = posixpath.normpath(target_path)
            target = parsed.get(target_path)
            if target is None:
                errors.append(f"{page_path}: missing page: {href}")
            elif target_url.fragment and unquote(target_url.fragment) not in target.ids:
                errors.append(f"{page_path}: missing anchor: {href}")
    return errors


def self_test() -> None:
    assert check({"index.html": '<a href="missing.html#lost">bad</a>'}) == [
        "index.html: missing page: missing.html#lost"
    ]
    assert check({"index.html": '<h2 id="日本語">節</h2><a href="#日本語">ok</a>'}) == []
    assert check(
        {"index.html": '<h2 id="日本語">節</h2><a href="#missing">bad</a>'}
    ) == ["index.html: missing anchor: #missing"]


if __name__ == "__main__":
    self_test()
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        print("link checker self-test passed")
        raise SystemExit(0)
    if len(sys.argv) != 2:
        raise SystemExit("usage: check-docs-links.py PUBLIC_DIR | --self-test")
    root = Path(sys.argv[1]).resolve()
    pages = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in root.rglob("*.html")
    }
    issues = check(pages)
    if "index.html" not in pages:
        issues.append("index.html: generated site entry is missing")
    if issues:
        print("\n".join(issues), file=sys.stderr)
        raise SystemExit(1)
    print(f"checked {len(pages)} generated pages and their internal links")
