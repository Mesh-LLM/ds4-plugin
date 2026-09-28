# CI inventory and topology

`verify.yml` runs the same serial package checks and released-host smoke for
pull requests and main. GitHub-hosted Ubuntu 24.04, contents-read only, no secrets,
no shared caches, no publishing. Superseded PRs cancel; main runs do not.
The runner supplies Rust and Python; no tool installers are executed.

`mesh.json` pins the released Linux host test dependency by version and SHA-256.
`fetch_test_host.py` downloads and verifies that dependency, not a mutable
installer. The smoke consumes that exact host and the package test-built plugin.
It installs a plugin archive with a fake native backend into a temporary HOME,
checks discovery/chat/tool replay/streaming through Mesh, shuts down the owned
host, and verifies the mock backend is gone. No weights/GPU, public mesh join,
real model inference, lab credentials or existing user configuration are used.

Real Apple Silicon runtime/weights acceptance is opt-in and separate. CI does
not claim model quality or native Metal qualification. The early-access macOS
archive has prior real-model evidence documented in README.

Validate workflow syntax with `actionlint`; run `just verify` and
`python3 scripts/mesh_smoke.py --mesh /path/to/mesh-llm --plugin target/debug/ds4`.
The parent Mesh repository's five-lane topology does not apply to this standalone
single-package plugin. This file owns its workflow/dependency inventory.
