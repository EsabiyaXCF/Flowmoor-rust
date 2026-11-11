#!/usr/bin/env python3
"""Check Git's public source candidates; optionally create a source-only ZIP."""
import argparse
import hashlib
from pathlib import Path
import re
import subprocess
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='Only validate; do not create an archive')
    args = parser.parse_args()
    names = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=ROOT).decode().split('\0')
    files = sorted(set(filter(None, names)))
    for required in ['Cargo.toml', 'Cargo.lock', 'LICENSE', 'README.md', 'run.sh', 'src/main.rs', 'src/cli.rs', 'third-party/THIRD_PARTY_NOTICES.html']:
        if required not in files:
            raise SystemExit(f'Missing source file: {required}')
    secret = re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|\bgh[pousr]_[A-Za-z0-9]{30,}|\bgithub_pat_[A-Za-z0-9_]{40,}|\bAKIA[A-Z0-9]{16}\b')
    personal = re.compile(rb'/Users/[A-Za-z0-9._-]+/|/private/var/folders/[A-Za-z0-9]+/|/home/[A-Za-z0-9._-]+/')
    payloads = {}
    for name in files:
        path = ROOT / name
        if path.is_symlink() or any(part in {'.tools', 'target', 'test-output', 'downloads', '.hallmark', '.git'} for part in path.relative_to(ROOT).parts):
            raise SystemExit(f'Non-source path in publication set: {name}')
        lower = path.name.lower()
        if lower.startswith('.env') and lower != '.env.example' or lower.endswith(('.pem', '.key')) or lower in {'settings.json', 'library.json'} or 'cookie' in lower and lower.endswith('.txt'):
            raise SystemExit(f'Private file in publication set: {name}')
        if path.suffix.lower() in {'.mp4', '.mkv', '.webm', '.mov', '.avi', '.flv', '.mp3', '.m4a', '.mka', '.opus', '.aac', '.flac', '.wav'}:
            raise SystemExit(f'Media file in source publication set: {name}')
        data = path.read_bytes()
        if data.startswith((b'\x7fELF', b'MZ', b'\xcf\xfa\xed\xfe', b'\xfe\xed\xfa\xcf', b'\xca\xfe\xba\xbe')):
            raise SystemExit(f'Executable binary in source publication set: {name}')
        if secret.search(data) or personal.search(data):
            raise SystemExit(f'Credential or personal path requires review: {name}')
        payloads[name] = data
    assert tomllib.loads(payloads['Cargo.toml'].decode())['package']['license'] == 'MIT'
    assert b'Copyright (c) 2026 Flowmoor' in payloads['LICENSE']
    print(f'Checked {len(files)} source files; no selected build/media/private files or matching credentials/personal paths.')
    if args.check:
        return
    version = tomllib.loads(payloads['Cargo.toml'].decode())['package']['version']
    stem = f'Flowmoor-rust-{version}'
    output = ROOT / 'test-output' / f'{stem}-source.zip'
    output.parent.mkdir(parents=True, exist_ok=True)
    manifest = ''.join(f'{hashlib.sha256(data).hexdigest()}  {name}\n' for name, data in payloads.items())
    with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in {**payloads, 'SOURCE-MANIFEST.sha256': manifest.encode()}.items():
            entry = zipfile.ZipInfo(f'{stem}/{name}', date_time=(1980, 1, 1, 0, 0, 0))
            entry.compress_type = zipfile.ZIP_DEFLATED
            mode = (ROOT / name).stat().st_mode & 0o777 if name in payloads else 0o644
            entry.external_attr = (0o100000 | mode) << 16
            archive.writestr(entry, data)
    print(f'Prepared {output.relative_to(ROOT)} ({output.stat().st_size} bytes); not committed, pushed or published.')


if __name__ == '__main__':
    main()
