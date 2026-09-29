# DwarfStar for Mesh

Run DeepSeek V4 Flash on your Mac and use it through Mesh’s normal API.
Mesh starts and stops the bundled [DwarfStar](https://github.com/antirez/ds4)
server for you. No compiler or separate server setup needed.

**Early access: Apple Silicon, Mesh v0.77.0.** Flash Q2 needs about 81 GiB
of disk space for weights and a 96 GB or larger Mac for resident inference.
Leave memory for macOS, context and other applications.

## 1. Install

```sh
mesh-llm plugins install Mesh-LLM/ds4-plugin@v0.1.0-trial.3
```

Mesh downloads and installs the plugin and native runtime. No manual extraction.
The macOS binaries are ad-hoc signed, not Apple-notarized.

For offline installation, download the archive from the
[release page](https://github.com/Mesh-LLM/ds4-plugin/releases/tag/v0.1.0-trial.3)
and use `mesh-llm plugins install --archive <file> --name ds4-plugin --version 0.1.0`.

## 2. Choose weights

Already have compatible DeepSeek V4 Flash Q2 weights? Skip to step 3.
Otherwise, review the model size/licence and explicitly download them:

```sh
~/.mesh-llm/plugins/installed/ds4-plugin/ds4-plugin catalog
~/.mesh-llm/plugins/installed/ds4-plugin/ds4-plugin download --model ds4f-q2 --directory "$HOME/Models/ds4" --accept-download
```

The download requires curl, resumes interrupted transfers and verifies SHA-256.
It prints the weight-file path; use that path below, without the `.partial`
suffix. Installing the plugin does not download weights, and downloading weights
does not start inference.

## 3. Start with Mesh

Add this to `~/.mesh-llm/config.toml`, replacing the weight path. If you already
have a `[runtime]` section, edit it rather than adding a second one.

```toml
[runtime]
mode = "on_demand"

[[plugin]]
name = "ds4-plugin"
args = ["serve", "--weights", "/absolute/path/to/model.gguf", "--context", "4096"]
```

```sh
mesh-llm serve
```

The model loads when Mesh starts the plugin—not on the first chat request.
Once ready, it appears in Mesh’s model list:

```sh
curl http://127.0.0.1:9337/v1/models
curl http://127.0.0.1:9337/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -d '{"model":"deepseek-v4-flash","messages":[{"role":"user","content":"Hello"}]}'
```

Use `deepseek-v4-flash` in your OpenAI-compatible client. Chat, streaming and
tool calls use the normal Mesh API. Upstream also lists a PRO alias; it refers
to the same loaded model, not a second model.

Ctrl+C in the Mesh terminal stops its plugin and backend. Remove the `ds4-plugin`
plugin entry to stop loading it on future launches. Weights stay in your model
directory.

## Notes

- Context defaults to 4096 tokens; `--context` accepts 512–32768.
- This download catalog currently contains Flash Q2 only. The user chooses
  the weights; Mesh does not automatically select or download them.
- A hard interruption during download can leave `.ds4-download.lock` in the
  model directory. Remove it only after confirming no download is running.
- The engine is MIT-licensed; weights have their own
  [model licence](https://huggingface.co/antirez/deepseek-v4-gguf).

## Development and testing

Use `just build`, `just verify`, and `just clean`. Default tests use fake
backends, never model weights. CI additionally installs the plugin into a
checksum-pinned released Mesh v0.77.0 and checks discovery, chat, tool replay,
streaming and shutdown cleanup with a mock backend.

For an already running instance with real weights:

```sh
just acceptance http://127.0.0.1:9337/v1
```

Trial.2 was tested with real Flash Q2 weights through Mesh on Apple Silicon,
including chat, streaming, tool replay and shutdown. CI does not load that model.

Maintainers build the plugin and pinned upstream server with
`just package-macos /path/to/clean/pinned/ds4`. The archive includes Metal
assets, upstream licence and `RUNTIME.sha256`; users do not compile either
program. Upstream revision: `0aaea5a238fb41a35106a551e73c8409dfb751ac`.
