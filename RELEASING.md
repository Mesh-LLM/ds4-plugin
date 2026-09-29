# Releasing

`mesh-llm plugins install Mesh-LLM/ds4-plugin` installs the asset
`ds4-plugin-aarch64-apple-darwin.tar.gz` (+ `.sha256`) from the **latest GitHub
release**, not `main`. Merging does not ship anything; a tag does.

1. PR bumping `version` in `Cargo.toml` and `plugin.toml` (and `Cargo.lock`,
   via `cargo check`). Merge it.
2. Tag the merge commit and push the tag:
   ```sh
   git tag v0.3.0 <merge-sha> && git push origin v0.3.0
   ```
   `.github/workflows/release.yml` (also run as a no-publish dry run on PRs that touch packaging) checks the tag matches the version, builds
   the plugin and the pinned upstream `ds4-server` on macOS, and publishes the
   release with the archive.
3. Check: `mesh-llm plugins install Mesh-LLM/ds4-plugin` in a clean `HOME`
   reports the new version.

Users update by re-running the same install command. To bump upstream ds4,
change `revision=` in `scripts/package-macos.sh`.

Manual fallback (on an Apple Silicon Mac, clean ds4 checkout at that revision):
`just package-macos /path/to/ds4`, then
`gh release create vX.Y.Z --target <sha> dist/ds4-plugin-aarch64-apple-darwin.tar.gz{,.sha256}`.
