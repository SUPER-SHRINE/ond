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


def check(pages: dict[str, str], ignored_missing_pages: set[str] | None = None) -> list[str]:
    parsed: dict[str, Page] = {}
    for path, source in pages.items():
        page = Page()
        page.feed(source)
        parsed[path] = page

    index_canonical = parsed.get("index.html", Page()).canonical
    base_path = urlsplit(index_canonical).path.rstrip("/") if index_canonical else ""
    if not index_canonical:
        for page_path, page in parsed.items():
            if not page.canonical:
                continue
            canonical_path = urlsplit(page.canonical).path.rstrip("/")
            page_suffix = "/" + page_path.removesuffix("index.html").rstrip("/")
            if page_path.endswith("index.html") and canonical_path.endswith(page_suffix):
                base_path = canonical_path[: -len(page_suffix)] if page_suffix != "/" else canonical_path
                break
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
                if ignored_missing_pages is None or target_path not in ignored_missing_pages:
                    errors.append(f"{page_path}: missing page: {href}")
            elif target_url.fragment and unquote(target_url.fragment) not in target.ids:
                errors.append(f"{page_path}: missing anchor: {href}")
    return errors


def expected_html_paths(sources: Path) -> set[str]:
    if not sources.is_dir():
        raise ValueError(f"source directory does not exist: {sources}")
    outputs: set[str] = set()
    for markdown in sources.rglob("*"):
        if not markdown.is_file() or markdown.suffix.lower() != ".md":
            continue
        relative = markdown.relative_to(sources)
        if relative.name.casefold() in {"index.md", "readme.md"}:
            output = relative.with_name("index.html")
        else:
            output = relative.with_suffix("") / "index.html"
        outputs.add(output.as_posix())
    if not outputs:
        raise ValueError(f"source directory contains no Markdown pages: {sources}")
    return outputs


def validate(pages: dict[str, str], expected: set[str] | None = None) -> tuple[int, list[str]]:
    missing = (expected or set()) - pages.keys()
    issues = check(pages, missing)
    if "index.html" not in pages and "index.html" not in missing:
        issues.append("index.html: generated site entry is missing")
    if issues:
        return 1, issues
    if missing:
        return 2, [f"missing generated page: {path}" for path in sorted(missing)]
    return 0, []


def self_test() -> None:
    assert check({"index.html": '<a href="missing.html#lost">bad</a>'}) == [
        "index.html: missing page: missing.html#lost"
    ]
    assert check({"index.html": '<h2 id="日本語">節</h2><a href="#日本語">ok</a>'}) == []
    assert check(
        {"index.html": '<h2 id="日本語">節</h2><a href="#missing">bad</a>'}
    ) == ["index.html: missing anchor: #missing"]

    root_missing = {
        "404.html": '<link rel="canonical" href="https://example.test/ond/404.html"><a href="/ond/guide/">guide</a>',
        "guide/index.html": '<link rel="canonical" href="https://example.test/ond/guide/">guide',
    }
    assert validate(root_missing, {"index.html"}) == (2, ["missing generated page: index.html"])
    assert validate(
        {**root_missing, "guide/index.html": '<link rel="canonical" href="https://example.test/ond/guide/"><a href="/ond/missing/">bad</a>'},
        {"index.html"},
    )[0] == 1
    assert validate(
        {**root_missing, "guide/index.html": '<link rel="canonical" href="https://example.test/ond/guide/"><a href="/ond/guide/#lost">bad</a>'},
        {"index.html"},
    )[0] == 1

    from tempfile import TemporaryDirectory

    test_root = Path(__file__).resolve().parent
    with TemporaryDirectory(dir=test_root) as directory:
        sources = Path(directory)
        for name in (
            "index.md",
            "README.md",
            "nested/index.md",
            "nested/README.md",
            "nested/guide.md",
            "unlinked/page.md",
        ):
            source = sources / name
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("page", encoding="utf-8")

        expected = expected_html_paths(sources)
        assert expected == {
            "index.html",
            "nested/index.html",
            "nested/guide/index.html",
            "unlinked/page/index.html",
        }
        pages = {
            "index.html": '<a href="nested/">nested</a>',
            "nested/index.html": '<a href="guide/">guide</a>',
            "nested/guide/index.html": '<h2 id="topic">topic</h2>',
            "unlinked/page/index.html": "unlinked",
        }
        assert validate(pages, expected) == (0, [])

        del pages["unlinked/page/index.html"]
        status, issues = validate(pages, expected)
        assert status == 2
        assert issues == ["missing generated page: unlinked/page/index.html"]

        broken_with_missing_output = {**pages, "index.html": '<a href="missing.html">bad</a>'}
        assert validate(broken_with_missing_output, expected)[0] == 1

        complete_pages = {**pages, "unlinked/page/index.html": "unlinked"}
        broken_page = {**complete_pages, "index.html": '<a href="missing.html">bad</a>'}
        broken_anchor = {
            **complete_pages,
            "nested/guide/index.html": '<a href="#missing">bad</a>',
        }
        assert validate(broken_page, expected)[0] == 1
        assert validate(broken_anchor, expected)[0] == 1

    with TemporaryDirectory(dir=test_root) as directory:
        try:
            expected_html_paths(Path(directory))
        except ValueError:
            pass
        else:
            raise AssertionError("an empty source directory must be rejected")


if __name__ == "__main__":
    self_test()
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        print("link checker self-test passed")
        raise SystemExit(0)
    if len(sys.argv) not in (2, 3):
        raise SystemExit("usage: check-docs-links.py PUBLIC_DIR [SOURCES_DIR] | --self-test")
    root = Path(sys.argv[1]).resolve()
    pages = {
        path.relative_to(root).as_posix(): path.read_text(encoding="utf-8")
        for path in root.rglob("*.html")
    }
    try:
        expected = expected_html_paths(Path(sys.argv[2]).resolve()) if len(sys.argv) == 3 else None
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        raise SystemExit(1) from error
    status, issues = validate(pages, expected)
    if issues:
        print("\n".join(issues), file=sys.stderr)
        raise SystemExit(status)
    print(f"checked {len(pages)} generated pages and their internal links")
