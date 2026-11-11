#!/usr/bin/env python3
"""Generate offline notices from the locked dependency graph and upstream texts."""

import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", help="Cargo target triple; defaults to rustc's host")
    parser.add_argument("--check", action="store_true", help="Fail if committed notices are stale")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    rustc = subprocess.check_output(["rustc", "-vV"], text=True)
    target = args.target or re.search(r"^host: (.+)$", rustc, re.M)[1]
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--offline", "--locked", "--format-version", "1",
        "--filter-platform", target,
    ], cwd=root, text=True))
    packages = {p["id"]: p for p in metadata["packages"]}
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    project = metadata["resolve"]["root"]
    resolved = set()
    pending = [project]
    while pending:
        package = pending.pop()
        if package not in resolved:
            resolved.add(package)
            pending.extend(d["pkg"] for d in nodes[package]["deps"])
    direct = {d["pkg"] for d in nodes[project]["deps"]}
    texts = {}

    def notice(text, label):
        key = "license-" + hashlib.sha256(text.encode()).hexdigest()[:20]
        texts.setdefault(key, (label, text))
        return key

    rows = []
    missing = []
    for package in sorted((packages[i] for i in resolved - {project}), key=lambda p: (p["name"], p["version"])):
        name, version = package["name"], package["version"]
        directory = Path(package["manifest_path"]).parent
        files = [f for f in sorted(directory.iterdir()) if f.is_file() and f.name.lower().startswith(("license", "licence", "copying", "notice", "copyright", "authors"))]
        if package["license_file"]:
            files.append(directory / package["license_file"])
        for folder in (directory / "LICENSES", directory / "licenses"):
            if folder.is_dir():
                files.extend(f for f in sorted(folder.rglob("*")) if f.is_file())
        override = root / "third-party/upstream" / f"{name}-{version}"
        sources = json.loads((override / "sources.json").read_text()) if (override / "sources.json").exists() else {}
        files.extend(f for f in sorted(override.glob("*")) if f.is_file() and f.name != "sources.json")
        links = []
        for file in dict.fromkeys(files):
            key = notice(file.read_text(), f"{name} {version} — {file.name}")
            links.append(f'<a href="#{key}">{html.escape(file.name)}</a>')
            if file.name in sources:
                links.append(f'<a href="{html.escape(sources[file.name], quote=True)}">上游原文 / upstream</a>')
        if not files:
            missing.append(f"{name} {version}")
            links.append("上游套件未附獨立授權檔；以下為套件宣告 / No standalone license text supplied; see declared license and source.")
        archive = f"https://static.crates.io/crates/{name}/{name}-{version}.crate"
        authors = "; ".join(package["authors"])
        repository = package["repository"]
        origin = f'<a href="{html.escape(repository, quote=True)}">Repository</a> · ' if repository else ""
        rows.append(
            f'<tr><td><strong>{html.escape(name)} {html.escape(version)}</strong>'
            f'<div>{html.escape(authors)}</div></td><td>{html.escape(package["license"] or "未宣告 / undeclared")}</td>'
            f'<td>{"直接 / direct" if package["id"] in direct else "間接或建置 / transitive or build"}</td>'
            f'<td>{origin}<a href="{archive}">此版本原始碼 / source archive</a><br>{"<br>".join(links)}</td></tr>'
        )

    resource_rows = []
    resource_files = [root / "LICENSE", root / "assets/hls.LICENSE", root / "assets/phosphor/LICENSE"]
    resource_files.extend(sorted((root / "third-party/licenses").glob("*.txt")))
    lucide = next(p for p in packages.values() if p["name"] == "gpui-kit-assets")
    resource_files.append(Path(lucide["manifest_path"]).parent / "LICENSE-LUCIDE")
    for file in resource_files:
        label = "Flowmoor Rust — MIT" if file == root / "LICENSE" else file.name if file.parent.name != "phosphor" else "Phosphor Icons — MIT"
        key = notice(file.read_text(), label)
        resource_rows.append(f'<li><a href="#{key}">{html.escape(label)}</a></li>')

    sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
    copyright_file = sysroot / "share/doc/rust/COPYRIGHT-library.html"
    if not copyright_file.exists():
        raise SystemExit("Rust standard-library notices are missing; install the rust-docs toolchain component before generating notices.")
    rust_notices = re.search(r"<body[^>]*>(.*)</body>", copyright_file.read_text(), re.S)[1]
    appendix = "\n".join(
        f'<details open id="{key}"><summary>{html.escape(label)}</summary><pre>{html.escape(text)}</pre></details>'
        for key, (label, text) in texts.items()
    )
    document = (root / "third-party/page.html").read_text()
    values = {
        "TARGET": html.escape(target),
        "LOCK_HASH": hashlib.sha256((root / "Cargo.lock").read_bytes()).hexdigest(),
        "RUST_VERSION": html.escape(rustc.splitlines()[0]),
        "PACKAGE_COUNT": str(len(rows)),
        "PACKAGES": "\n".join(rows),
        "RESOURCE_LICENSES": "\n".join(resource_rows),
        "LICENSE_TEXTS": appendix,
        "RUST_NOTICES": rust_notices,
        "MISSING_TEXTS": html.escape(", ".join(missing)) or "無 / None",
    }
    for key, value in values.items():
        marker = f"<!-- {key} -->"
        assert marker in document, f"Missing template marker: {key}"
        document = document.replace(marker, value)
    assert "<!-- " not in document, "Unresolved notice template marker"
    output = root / "third-party/THIRD_PARTY_NOTICES.html"
    if args.check:
        if not output.exists() or output.read_text() != document:
            raise SystemExit("Third-party notices are stale; run python3 scripts/generate-third-party-notices.py with the same target/toolchain.")
    else:
        output.write_text(document)
    print(f"{'Checked' if args.check else 'Generated'} {len(rows)} packages, {len(texts)} unique texts for {target}; {len(missing)} upstream texts unavailable.")


if __name__ == "__main__":
    main()
