# ds4 — managed DwarfStar plugin for Mesh

**Development prototype, not an installable release or validated inference integration.**
Standalone native plugin following [Flash-MoE](https://github.com/Mesh-LLM/flash-moe).
No engine fork, Skippy change, or inference-over-plugin-IPC path.

## Current prototype

- Explicit `ds4 catalog` and opt-in, resumable `ds4 download` for V4 Flash Q2.
- Immutable Hugging Face revision, exact byte count and SHA-256 verification.
- `serve` accepts a provisioned runtime directory and existing weights; owns only
  its loopback child and retains weights on exit.
- One session, default 4096 context; no model download on startup.
- Runtime must currently be provisioned separately. **Precompiled runtime fetch
  is not implemented yet.** No catalog registration or release has been published.

```sh
ds4 catalog
ds4 download --model ds4f-q2 --directory /absolute/path/to/models --accept-download
```

Download requires curl. It never starts inference. Cancelled/failed downloads
retain `.partial` data. A hard interruption can leave `.ds4-download.lock`;
remove it only after confirming no download is running. Existing final files
are verified, never silently overwritten. Store weights outside the plugin
installation directory so plugin deletion does not remove them.

Proposed development configuration (not yet live-qualified):

```toml
[runtime]
mode = "on_demand"

[[plugin]]
name = "ds4"
command = "/absolute/path/to/ds4"
args = ["serve", "--runtime", "/absolute/path/to/runtime", "--weights", "/absolute/path/to/models/model.gguf", "--context", "4096"]
```

The runtime directory must contain `ds4-server` and the matching runtime Metal
assets. This prototype targets upstream
`0aaea5a238fb41a35106a551e73c8409dfb751ac`. Do not interpret a successful
`/v1/models` probe as checkpoint verification or tool-use certification.

## Before release

1. Resolve canonical model discovery: upstream V4 `/v1/models` advertises both
   Flash and PRO aliases regardless of which checkpoint is loaded. Do not ship
   this as two separately available models. Establish a generic host metadata
   filtering contract or an upstream correction; no ds4-specific core branches.
2. Complete startup handshake/readiness integration, graceful SIGTERM and
   shutdown coverage, collision handling and lifecycle tests against fake IPC/HTTP.
3. Publish reproducible, pinned Apple Silicon runtime bundles including shaders,
   upstream MIT licence and provenance. Implement verified staged runtime install
   and rollback. Never download/execute mutable unverified runtime scripts.
4. Add disk/RAM preflight, cancellation recovery, and download fault-injection
   tests. No automatic unloading of other workloads.
5. Verify direct/local-Mesh/remote-private-Mesh streaming and real agent tool
   loops, backend death/withdrawal, cancellation and context limits. Only then
   publish native plugin archives and add the catalog entry.

Engine licence and model licence are separate. Model source:
https://huggingface.co/antirez/deepseek-v4-gguf/tree/f71f23d552d664e523b422157b2befbf74040380

## Development

`just build`, `just verify`, `just clean`. Tests do not load weights or start an
inference server. SDK is pinned to a commit, Cargo dependencies to Cargo.lock.
