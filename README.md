# DwarfStar for Mesh

Run [DwarfStar (ds4)](https://github.com/antirez/ds4) models through Mesh's
normal OpenAI-compatible API. The plugin bundles the engine, fetches the model
you name, and starts and stops the server with Mesh.

**Early access: Apple Silicon, Mesh v0.77.0+.** Models are large (DeepSeek V4
Flash Q2 is ~81 GiB); pick one that fits your Mac's memory.

## Setup

```sh
mesh-llm plugins install Mesh-LLM/ds4-plugin
```

Add to `~/.mesh-llm/config.toml`:

```toml
[runtime]
mode = "on_demand"   # don't also load a built-in model next to DwarfStar

[[plugin]]
name = "ds4-plugin"
args = ["serve", "--model", "ds4f-q2"]
```

Then run `mesh-llm serve`. The first start downloads the model (progress shows
in the terminal) and later starts reuse it. Once it's loaded, the model appears
in `curl http://127.0.0.1:9337/v1/models`. Use that id in any OpenAI-compatible
client.

## Details

- `--model` takes any name upstream's downloader accepts, e.g. `ds4f-q2`,
  `ds4f-q4`, `ds41f-q2`, `glm53-q2`, `qwen38-q4k`. See
  [ds4 models](https://github.com/antirez/ds4/blob/main/docs/MODELS.md).
  Only `ds4f-q2` has been tested through Mesh so far.
- Already have weights? Use `"--weights", "/path/to/model.gguf"` instead of `--model`.
- Weights are stored in `~/.mesh-llm/models/ds4` (change with `--model-dir`).
  They are kept when you stop, uninstall or upgrade the plugin.
- Context uses ds4-server's default (32768); set `"--context", "65536"` to change.
- An interrupted download resumes on the next start. Some models (PRO, MXFP4)
  need the Hugging Face CLI: `python3 -m pip install -U huggingface_hub hf_xet`.
  `HF_TOKEN` is honoured.
- Config changes take effect when you restart Mesh. Ctrl+C stops the plugin and
  the engine.
- Mesh routes to DwarfStar as one local endpoint; it does not split the model
  across Mesh nodes.

## Development

`just build`, `just verify`, `just clean`. Default tests use fake backends and a
fake downloader, never real weights. `just acceptance http://127.0.0.1:9337/v1`
probes an already running instance. Maintainers package with
`just package-macos /path/to/clean/pinned/ds4` (upstream revision
`0aaea5a238fb41a35106a551e73c8409dfb751ac`); the archive includes `ds4-server`,
Metal assets, upstream `download_model.sh` and its licence.
