#!/usr/bin/env python3
"""原稿のタイトル付きfile例を抽出し、記事どおりcompile/linkする。"""
from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[1]
MANUAL = ROOT / "docs" / "manual"
TARGET = ROOT / "target"
GENERATED = TARGET / "manual-tests"


@dataclass(frozen=True)
class Case:
    article: str
    folder: str = "."
    role: str | None = None
    failure: bool = False
    trap_reason: str | None = None


CASES = [
    Case("getting-started/first-program.md"),
    Case("tutorials/projects-and-packages.md"),
    Case("tutorials/libraries-and-dependencies.md", "app"),
    Case("tutorials/roles.md", role="common"),
    Case("tutorials/roles.md", role="Debug"),
    Case("tutorials/roles.md", role="Development"),
    Case("tutorials/diagnostics.md", "broken", failure=True),
    Case("tutorials/diagnostics.md", "fixed", trap_reason="count must be nonzero"),
]


TITLE = re.compile(r'\btitle="([^"]+)"')
LOCATION = re.compile(r"main\.ond:\d+(?::\d+)?")


def parse_article(article: str, destination: Path) -> None:
    """title付きfenceだけを、記事内の相対fileとして保存する。"""
    source = MANUAL / article
    lines = source.read_text(encoding="utf-8-sig").splitlines(keepends=True)
    index = 0
    found: set[str] = set()
    while index < len(lines):
        line = lines[index]
        if not line.startswith("```"):
            index += 1
            continue
        match = TITLE.search(line)
        marker = "`" * (len(line) - len(line.lstrip("`")))
        index += 1
        body: list[str] = []
        while index < len(lines) and not lines[index].startswith(marker):
            body.append(lines[index])
            index += 1
        if index >= len(lines):
            raise ValueError(f"{source}: fenced code block is not closed")
        index += 1
        if not match:
            continue
        name = match.group(1)
        relative = PurePosixPath(name)
        if relative.is_absolute() or ".." in relative.parts or "\\" in name:
            raise ValueError(f"{source}: unsafe title path: {name}")
        output = (destination / Path(*relative.parts)).resolve()
        try:
            output.relative_to(destination.resolve())
        except ValueError as exc:
            raise ValueError(f"{source}: title escapes generated project: {name}") from exc
        key = relative.as_posix()
        if key in found:
            raise ValueError(f"{source}: duplicate file title: {name}")
        found.add(key)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text("".join(body), encoding="utf-8", newline="\n")
    if not found:
        raise ValueError(f"{source}: no titled example files found")


def generated_root() -> Path:
    target_root = TARGET.resolve()
    generated = GENERATED.resolve()
    if generated != target_root / "manual-tests" or not generated.is_relative_to(target_root):
        raise RuntimeError(f"unsafe generated path: {generated}")
    return generated


def output_path(project: Path, configured: str, label: str) -> Path:
    path = (project / configured).resolve()
    project_target = (project / "target").resolve()
    try:
        path.relative_to(project_target)
        path.relative_to(generated_root())
    except ValueError as exc:
        raise RuntimeError(f"{label} output is outside generated target/: {path}") from exc
    return path


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )


def check_help(ond: Path) -> str:
    result = run([str(ond), "--help"], ROOT)
    if result.returncode != 0 or "compile" not in result.stdout or "link" not in result.stdout:
        raise RuntimeError(f"CLI --help failed ({result.returncode})\n{result.stdout}\n{result.stderr}")
    return "installation: CLI --help and compile/link usage"


def check_case(ond: Path, project: Path, case: Case) -> str:
    if not project.is_dir() or not (project / "ond.toml").is_file():
        raise RuntimeError(f"{case.article}/{case.folder}: ond.toml is missing")
    config = tomllib.loads((project / "ond.toml").read_text(encoding="utf-8-sig"))
    compile_config = config.get("compile", {})
    link_config = config.get("link", {})
    compile_output = output_path(project, compile_config.get("output", "target/build.ondbuild"), "compile")
    outputs = [compile_output]
    binary = None
    sidecar = None
    if not case.failure:
        link_name = link_config.get("output")
        if not isinstance(link_name, str):
            raise RuntimeError(f"{case.article}/{case.folder}: [link].output is required")
        binary = output_path(project, link_name, "link")
        sidecar = binary.with_suffix(".onddebug")
        outputs.extend([binary, sidecar])
    for output in outputs:
        output.unlink(missing_ok=True)

    command = [str(ond), "compile", "."]
    if case.role not in (None, "common"):
        command.extend(["--role", case.role])
    result = run(command, project)
    if case.failure:
        if result.returncode != 1:
            raise RuntimeError(
                f"{case.article}/{case.folder}: expected compile exit 1, got {result.returncode}\n{result.stderr}"
            )
        if not LOCATION.search(result.stderr):
            raise RuntimeError(
                f"{case.article}/{case.folder}: diagnostic has no main.ond line/column\n{result.stderr}"
            )
        return f"{case.article}/{case.folder}: expected diagnostic at source position"
    if result.returncode != 0:
        raise RuntimeError(f"{case.article}/{case.folder}: compile failed ({result.returncode})\n{result.stderr}")
    if not compile_output.is_file():
        raise RuntimeError(f"{case.article}/{case.folder}: configured compile output is missing: {compile_output}")

    result = run([str(ond), "link", "--project", "."], project)
    if result.returncode != 0:
        raise RuntimeError(f"{case.article}/{case.folder}: link failed ({result.returncode})\n{result.stderr}")
    if binary is None or sidecar is None or not binary.is_file() or not sidecar.is_file():
        raise RuntimeError(f"{case.article}/{case.folder}: configured binary or .onddebug sidecar is missing")
    if case.trap_reason and case.trap_reason.encode("utf-8") not in sidecar.read_bytes():
        raise RuntimeError(f"{case.article}/{case.folder}: trap reason missing from .onddebug sidecar")
    detail = " (trap reason in sidecar)" if case.trap_reason else ""
    return f"{case.article}/{case.folder}: article outputs and sidecar{detail}"


def main() -> int:
    parser = argparse.ArgumentParser(description="Manuscript example compile/link checks")
    parser.add_argument("-Ond", type=Path, help="path to the current Ond CLI")
    args = parser.parse_args()
    ond = args.Ond if args.Ond else ROOT / "target" / "debug" / ("ond.exe" if os.name == "nt" else "ond")
    ond = ond.resolve()
    if not ond.is_file():
        print(f"Ond CLI not found: {ond}; build it or pass -Ond", file=sys.stderr)
        return 2
    if sys.version_info < (3, 11):
        print("Python 3.11 or newer is required (stdlib tomllib)", file=sys.stderr)
        return 2

    try:
        generated = generated_root()
        if generated.exists():
            shutil.rmtree(generated)
        generated.mkdir(parents=True)
        tour = [Case(path.relative_to(MANUAL).as_posix()) for path in sorted((MANUAL / "language-tour").rglob("*.md"))]
        role_article = "language-tour/configuration/roles.md"
        role_cases = [Case(role_article, role="Debug"), Case(role_article, role="Development")]
        cases = [*CASES, *tour, *role_cases]
        articles = sorted({case.article for case in cases})
        article_roots = {
            article: generated / f"case-{index:03}"
            for index, article in enumerate(articles, start=1)
        }
        for article in articles:
            parse_article(article, article_roots[article])
        template = article_roots["getting-started/first-program.md"] / "ond.toml"
        for case in tour:
            project = article_roots[case.article]
            if not (project / "main.ond").is_file():
                raise ValueError(f"{case.article}: main.ond is required")
            if not (project / "ond.toml").is_file():
                shutil.copyfile(template, project / "ond.toml")
        results = [check_help(ond)]
        for case in cases:
            article_root = article_roots[case.article]
            project = article_root if case.folder == "." else article_root / case.folder
            results.append(check_case(ond, project, case))
        for result in results:
            print(f"ok: {result}")
        print(f"{len(results)} manual checks passed")
        return 0
    except (OSError, ValueError, RuntimeError) as error:
        print(f"manual example check failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())

