#!/usr/bin/env python3
"""Compose the GitHub-installable ds4-plugin archive from a built ds4 bundle."""
import argparse
import hashlib
from pathlib import Path
import shutil
import tarfile
import tempfile


def compose(source, output):
    with tempfile.TemporaryDirectory(prefix='ds4-package-') as temporary:
        root = Path(temporary)
        with tarfile.open(source) as archive:
            archive.extractall(root, filter='data')
        package = root / 'ds4'
        # Mesh resolves a GitHub reference by repository name, including its executable.
        shutil.copy2(package / 'ds4', package / 'ds4-plugin')
        manifest = package / 'plugin.toml'
        manifest.write_text(manifest.read_text().replace('name = "ds4"', 'name = "ds4-plugin"')
                            .replace('command = "ds4"', 'command = "ds4-plugin"'))
        with tarfile.open(output, 'w:gz') as archive:
            archive.add(package, arcname='ds4-plugin')
    output.with_name(output.name + '.sha256').write_text(
        hashlib.sha256(output.read_bytes()).hexdigest() + '  ' + output.name + '\n')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    compose(args.source, args.output)
