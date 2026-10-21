#!/usr/bin/env python3
"""Explicit development-only fixture download. Never used by Companion at runtime."""
import argparse
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / 'server/validation/syzygy-fixtures.json'


def prepare(output, fetch=urllib.request.urlopen):
    manifest = json.loads(MANIFEST.read_text(encoding='utf-8'))
    output.mkdir(parents=True, exist_ok=True)
    for item in manifest['files']:
        if Path(item['name']).name != item['name'] or '/' in item['name'] or '\\' in item['name']:
            raise ValueError('Invalid fixture filename')
        path = output / item['name']
        if path.is_symlink() or path.with_suffix(path.suffix + '.partial').is_symlink():
            raise ValueError('Fixture paths cannot be symlinks')
        if path.exists() and hashlib.sha256(path.read_bytes()).hexdigest() == item['sha256']:
            continue
        # Exact HTTPS URLs and hashes are pinned in the reviewed manifest.
        if not item['url'].startswith('https://tablebase.lichess.ovh/tables/standard/'):
            raise ValueError('Unapproved fixture source')
        with fetch(item['url'], timeout=30) as response:
            data = response.read(item['bytes'] + 1)
        if len(data) != item['bytes'] or hashlib.sha256(data).hexdigest() != item['sha256']:
            raise ValueError(f"Fixture size/hash mismatch: {item['name']}")
        if data[-16:].hex() != item['embeddedChecksum']:
            raise ValueError(f"Embedded checksum mismatch: {item['name']}")
        temporary = path.with_suffix(path.suffix + '.partial')
        temporary.write_bytes(data)
        temporary.replace(path)
    print(f"Verified {len(manifest['files'])} fixtures ({sum(f['bytes'] for f in manifest['files']):,} bytes).")


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'tmp/companion-tablebase-validation/tables')
    prepare(parser.parse_args().output)
