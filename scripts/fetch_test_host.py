#!/usr/bin/env python3
"""Fetch the manifest-pinned integration test dependency, never an installer."""
import hashlib
import json
from pathlib import Path
import tarfile
import urllib.request

root = Path(__file__).resolve().parent.parent
spec = json.loads((root / 'ci/mesh.json').read_text())
output = root / 'target/test-host'
output.mkdir(parents=True, exist_ok=True)
archive = output / 'host.tar.gz'
with urllib.request.urlopen(spec['url'], timeout=120) as response, archive.open('wb') as dest:
    while chunk := response.read(1024 * 1024):
        dest.write(chunk)
assert hashlib.sha256(archive.read_bytes()).hexdigest() == spec['sha256'], 'host checksum mismatch'
with tarfile.open(archive) as bundle:
    bundle.extractall(output, filter='data')
print(output / 'mesh-bundle/mesh-llm')
