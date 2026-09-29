# ds4 plugin

Standalone Mesh plugin, not an engine fork. Follow Mesh-LLM/openai-endpoint and
Mesh-LLM/flash-moe packaging: native executable plus plugin.toml.

- Work on branches/worktrees. Never push or merge main.
- Use `just build`, `just verify`, `just clean`; run Cargo serially.
- Test the whole package. Never start a real model during default tests.
- Never stop unrelated processes, change Mesh config, or download weights on startup.
- Setup/download must be explicit. Retain weights on shutdown/uninstall.
- Keep inference on the direct HTTP data plane; plugin IPC is control only.
- Read mesh-llm's manage-ci skill before CI changes.
- Commits use configured identity; include implementing agent Co-authored-by and
  configured identity Signed-off-by. Do not use another identity's signing key.
