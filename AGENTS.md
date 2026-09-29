# ds4 plugin

Standalone Mesh plugin, not an engine fork. Follow Mesh-LLM/openai-endpoint and
Mesh-LLM/flash-moe packaging: native executable plus plugin.toml.

- Work on branches/worktrees. Never push or merge main.
- Use `just build`, `just verify`, `just clean`; run Cargo serially.
- Test the whole package. Never start a real model during default tests.
- Never stop unrelated processes or change Mesh config.
- Download only the model the user named in `serve --model`, via the bundled
  upstream `download_model.sh`; no bespoke downloader or catalog. Retain weights
  on shutdown/uninstall. Never download in default tests.
- Keep inference on the direct HTTP data plane; plugin IPC is control only.
- Read mesh-llm's manage-ci skill before CI changes.
- Commits use configured identity; include implementing agent Co-authored-by and
  configured identity Signed-off-by. Do not use another identity's signing key.
