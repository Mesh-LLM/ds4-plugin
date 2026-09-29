import hashlib
from pathlib import Path
import tarfile
import tempfile
import unittest
from github_package import compose


class GitHubPackageTests(unittest.TestCase):
    def test_composition_preserves_binaries_and_matches_repo_name(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'ds4'
            source.mkdir()
            (source / 'ds4').write_bytes(b'unchanged signed executable')
            (source / 'ds4').chmod(0o755)
            (source / 'plugin.toml').write_text('name = "ds4"\ncommand = "ds4"\n')
            (source / 'runtime').mkdir()
            (source / 'runtime/server').write_bytes(b'unchanged runtime')
            original, output = root / 'original.tar.gz', root / 'ds4-plugin-test.tar.gz'
            with tarfile.open(original, 'w:gz') as archive:
                archive.add(source, arcname='ds4')
            compose(original, output)
            with tarfile.open(output) as archive:
                self.assertEqual(archive.extractfile('ds4-plugin/ds4-plugin').read(),
                                 (source / 'ds4').read_bytes())
                self.assertEqual(archive.getmember('ds4-plugin/ds4-plugin').mode & 0o777, 0o755)
                self.assertEqual(archive.extractfile('ds4-plugin/runtime/server').read(),
                                 b'unchanged runtime')
                self.assertIn(b'name = "ds4-plugin"',
                              archive.extractfile('ds4-plugin/plugin.toml').read())
            self.assertEqual(output.with_name(output.name + '.sha256').read_text().split()[0],
                             hashlib.sha256(output.read_bytes()).hexdigest())
