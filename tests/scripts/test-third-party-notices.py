#!/usr/bin/env python3
"""Check offline notice integrity and the macOS packaging path without launching UI."""

from html.parser import HTMLParser
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
subprocess.run(["python3", str(root / "scripts/generate-third-party-notices.py"), "--check"], check=True)
document = (root / "third-party/THIRD_PARTY_NOTICES.html").read_text()


class Notices(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = set()
        self.fragments = set()
        self.rows = 0
        self.in_packages = False
        self.runtime_requests = []

    def handle_starttag(self, tag, attributes):
        attributes = dict(attributes)
        if "id" in attributes:
            assert attributes["id"] not in self.ids, attributes["id"]
            self.ids.add(attributes["id"])
        href = attributes.get("href", "")
        if href.startswith("#"):
            self.fragments.add(href[1:])
        if tag in ("script", "iframe", "img", "link") or "src" in attributes:
            self.runtime_requests.append((tag, attributes))
        if tag == "tbody":
            self.in_packages = True
        if tag == "tr" and self.in_packages:
            self.rows += 1

    def handle_endtag(self, tag):
        if tag == "tbody":
            self.in_packages = False


page = Notices()
page.feed(document)
assert not page.runtime_requests, page.runtime_requests
assert page.fragments <= page.ids, page.fragments - page.ids
assert page.rows > 0
for required in ("hls.js light 1.6.15", "Phosphor", "Feather", "option-ext", "MPL-2.0", "GNU GENERAL PUBLIC LICENSE", "2026.10.26.220202", "Twitch", "YouTube"):
    assert required in document, required
assert "<!-- " not in document

# Exercise the actual packager with disposable executables; no GUI or downloads.
(root / "test-output").mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix="notices-package-", dir=root / "test-output") as temporary:
    fixture = Path(temporary)
    shutil.copy2(root / "run.sh", fixture / "run.sh")
    (fixture / "assets").symlink_to(root / "assets", target_is_directory=True)
    (fixture / "third-party").symlink_to(root / "third-party", target_is_directory=True)
    (fixture / "scripts").mkdir()
    (fixture / "scripts/generate-third-party-notices.py").write_text("from pathlib import Path\nPath('generated').touch()\n")
    for profile in ("debug", "release"):
        (fixture / f"target/{profile}").mkdir(parents=True)
        (fixture / f"target/{profile}/flowmoor-rust").write_text("#!/bin/sh\nexit 0\n")
        (fixture / f"target/{profile}/flowmoor-rust").chmod(0o755)
    commands = fixture / ".cargo/bin"
    commands.mkdir(parents=True)
    for name, command in {"cargo": 'case "$1" in fetch|build) exit 0 ;; *) exit 1 ;; esac\n', "uname": "echo Darwin\n"}.items():
        (commands / name).write_text("#!/bin/sh\n" + command)
        (commands / name).chmod(0o755)
    for flags in ([], ["--release"]):
        subprocess.run(["sh", str(fixture / "run.sh"), *flags], env={**os.environ, "HOME": str(fixture)}, check=True)
    assert (fixture / "generated").is_file()
    bundled = fixture / "target/Flowmoor Rust.app/Contents/Resources/THIRD_PARTY_NOTICES.html"
    assert bundled.read_text() == document

print(f"Third-party notices: {page.rows} package rows, all anchors valid, no runtime network resources, macOS bundle contains the full document.")
