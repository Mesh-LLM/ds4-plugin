#!/usr/bin/env python3
"""Install this plugin into an isolated released Mesh host and exercise its API."""
import argparse
import json
import os
from pathlib import Path
import shlex
import shutil
import socket
import subprocess
import sys
import tarfile
import tempfile
import time
from acceptance import check, request


def port():
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mesh', type=Path, required=True)
    parser.add_argument('--plugin', type=Path, required=True)
    args = parser.parse_args()
    mesh, plugin = args.mesh.resolve(), args.plugin.resolve()
    scripts = Path(__file__).resolve().parent
    with tempfile.TemporaryDirectory(prefix='ds4-smoke-', dir='/tmp') as temporary:
        root = Path(temporary)
        home = root / 'home'
        home.mkdir()
        package = root / 'ds4'
        package.mkdir()
        shutil.copy2(plugin, package / 'ds4')
        shutil.copy2(scripts.parent / 'plugin.toml', package / 'plugin.toml')
        runtime = package / 'runtime'
        runtime.mkdir()
        launcher = runtime / 'ds4-server'
        launcher.write_text('#!/bin/sh\nexec ' + shlex.quote(sys.executable) + ' ' +
                            shlex.quote(str(scripts / 'mock_backend.py')) + ' "$@"\n')
        launcher.chmod(0o755)
        weights = root / 'fake.gguf'
        weights.write_text('test fixture, not weights')
        archive = root / 'plugin.tar.gz'
        with tarfile.open(archive, 'w:gz') as output:
            output.add(package, arcname='ds4')
        env = {key: value for key, value in os.environ.items()
               if not key.startswith(('MESH_', 'BUZZ_', 'XDG_'))}
        env.update(HOME=str(home), XDG_CONFIG_HOME=str(home / '.config'),
                   XDG_CACHE_HOME=str(home / '.cache'), PYTHONDONTWRITEBYTECODE='1',
                   MESH_LLM_OWNER_PASSPHRASE='isolated-test-only')
        subprocess.run([str(mesh), '--log-format', 'json', 'plugins', 'install',
                        '--archive', str(archive), '--name', 'ds4', '--version', '0.1.0'],
                       env=env, check=True, timeout=60)
        config = home / '.mesh-llm' / 'config.toml'
        config.parent.mkdir(exist_ok=True)
        config.write_text('[runtime]\nmode = "on_demand"\n\n[[plugin]]\nname = "ds4"\n'
                          'args = ["serve", "--weights", ' + json.dumps(str(weights)) + ']\n')
        api, console = port(), port()
        base = f'http://127.0.0.1:{api}/v1'
        log = root / 'host.log'
        with log.open('w') as output:
            host = subprocess.Popen([str(mesh), 'serve', '--port', str(api), '--console',
                                     str(console), '--log-format', 'json'], env=env,
                                    stdin=subprocess.DEVNULL, stdout=output, stderr=output)
            try:
                deadline = time.monotonic() + 90
                while True:
                    if host.poll() is not None:
                        raise RuntimeError(f'host exited: {host.returncode}')
                    try:
                        models = json.loads(request(base, '/models'))
                        if any(m['id'] == 'deepseek-v4-flash' for m in models['data']):
                            break
                    except (OSError, ValueError):
                        pass
                    if time.monotonic() > deadline:
                        raise TimeoutError('plugin model not discovered')
                    time.sleep(1)
                print(json.dumps(check(base, 'deepseek-v4-flash'), indent=2))
            except BaseException:
                print(log.read_text(), file=sys.stderr)
                raise
            finally:
                host.terminate()
                try:
                    host.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    host.kill()
                    host.wait(timeout=5)
                time.sleep(2)
                for pidfile in home.rglob('backend.pid'):
                    pid = int(pidfile.read_text())
                    try:
                        os.kill(pid, 0)
                    except ProcessLookupError:
                        continue
                    raise RuntimeError(f'owned backend survived host shutdown: {pid}')
        print('Released Mesh discovery/chat/tools/stream/shutdown: PASS')


if __name__ == '__main__':
    main()
