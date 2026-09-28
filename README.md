# ds4 — managed DwarfStar plugin for Mesh

**Apple Silicon trial; not production-certified.** Standalone native plugin
following [Flash-MoE](https://github.com/Mesh-LLM/flash-moe). No engine fork,
Skippy change, or inference-over-plugin-IPC path.

## Try the precompiled archive

Extract `ds4-v0.1.0-aarch64-apple-darwin.tar.gz`. It contains the plugin,
`runtime/ds4-server`, Metal shader assets and upstream licence. No compiler is
needed. Keep this directory; weights should live elsewhere. The bundle is
ad-hoc signed, not Apple-notarized. Tested on one Apple Silicon machine only.

```sh
# Review the catalog (does not download):
./ds4/ds4 catalog
# Optional: explicitly download ~81 GiB, resumably, then verify SHA-256:
./ds4/ds4 download --model ds4f-q2 --directory /absolute/models --accept-download
# Or reuse existing compatible Flash Q2 weights. Starts one session:
./ds4/ds4 serve --standalone --weights /absolute/models/model.gguf
```

The last command prints the allocated loopback `/v1` URL and a Ready line.
Use `deepseek-v4-flash` as the model. Ctrl+C stops its owned server, not other
instances. Default context is 4096; `--context` accepts 512–32768. Allow ~81 GiB
for resident weights plus context, OS and other applications. Do not run beside
another large resident model. No automatic unloading or model download occurs.

## Run through Mesh

With Mesh v0.77.0 (plugin protocol 3), install the trial.2 archive:

```sh
mesh-llm plugins install --archive ./ds4-v0.1.0-aarch64-apple-darwin.tar.gz --name ds4 --version 0.1.0
```

Configure `~/.mesh-llm/config.toml` and launch `mesh-llm serve`:

```toml
[runtime]
mode = "on_demand"

[[plugin]]
name = "ds4"
args = ["serve", "--weights", "/absolute/models/model.gguf", "--context", "4096"]
```

The plugin finds its adjacent runtime automatically; `--runtime` can override it.
SDK pinned at `4ae1ace57dbbe28d0c3d10a05ee542328e8e64e7` (v0.77.0).
Trial.2 passed real archive installation into released Mesh v0.77.0, model
discovery, chat, a two-turn tool-call replay, streaming content/SSE termination,
and host shutdown with no surviving owned processes. Private remote routing
and full agent-harness qualification remain pending. Trial.1 used protocol 2
and is incompatible with this host; use trial.2 instead.

Initialization rejects incompatible hosts before starting a model. A separate
Unix watchdog reaps the native backend even when Mesh force-kills the plugin.
Automated tests cover incompatible initialization, host disconnect and plugin
SIGKILL while leaving unrelated processes alive. This trial targets Apple
Silicon/macOS; Windows supervision is not implemented.

**Known discovery limitation:** upstream lists Flash and PRO as compatibility
aliases for one loaded V4 checkpoint. Use Flash explicitly; do not interpret the
PRO alias as another loaded model. We have not added a proxy or engine fork to
hide this. Direct streaming framing passed, but response quality was not asserted.

## Download and provenance

Download requires curl. It never starts inference. Cancelled/failed downloads
retain `.partial` data. A hard interruption can leave `.ds4-download.lock`;
remove it only after confirming no download is running. Existing final files
are verified, never silently overwritten. Keep weights outside the plugin
installation directory so deletion does not remove them.

Weights use an immutable Hugging Face revision, exact size and SHA-256.
Bundled upstream runtime: `0aaea5a238fb41a35106a551e73c8409dfb751ac`, built
without host-native CPU tuning. `RUNTIME.sha256` inventories runtime assets;
the archive has a checksum sidecar. Engine MIT licence is included separately.

## Before release

1. Resolve canonical model discovery: upstream V4 `/v1/models` advertises both
   Flash and PRO aliases regardless of which checkpoint is loaded. Do not ship
   this as two separately available models. Establish a generic host metadata
   filtering contract or an upstream correction; no ds4-specific core branches.
2. Complete startup handshake/readiness integration, graceful SIGTERM and
   shutdown coverage, collision handling and lifecycle tests against fake IPC/HTTP.
3. Automate release publication and verified download/install/rollback of the
   composed archive. Never download/execute mutable unverified runtime scripts.
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

Maintainers: `just package-macos /path/to/clean/pinned/ds4` builds the composed
trial archive and cleans upstream build outputs. Users do not run this step.
