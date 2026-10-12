# Canto

A desktop studio for making songs with [YuE2](https://github.com/vanch007/mlx-Yue) on your own Mac. Describe a song, let Claude or Codex write the title, style and lyrics, and YuE2 sings it locally. Prompt refinement runs through the `claude` or `codex` CLI, so it uses your subscription, not API credits.

Canto has three parts over one core:

- **`canto-core`** (library): the song library, the YuE2 engine adapter, and the refiner adapters.
- **`canto`** (CLI): the agent surface. Every command takes `--json`.
- **`canto-app`** (GPUI app): the human surface, built with [GPUI](https://gpui.rs) and [GPUI Kit](https://gpui-kit.com).

YuE2 weights are CC-BY-NC-4.0, so songs are for personal, non-commercial use.

## Install

You need Rust, Xcode (for GPUI's Metal shaders), a working mlx-Yue install, and `claude` and/or `codex` on `PATH`. `ffmpeg` is needed only for MP3 export.

```bash
make install
```

This builds `~/Applications/Canto.app` and installs the CLI to `~/.local/bin/canto`. The `.cargo/config.toml` points `DEVELOPER_DIR` at Xcode, because the Command Line Tools have no Metal compiler.

## Use the app

- **Simple** — describe the song in a sentence and press **Write & create**. The refiner writes the brief, fills the Custom form, and queues the song. **Write it, but let me edit first** stops after the writing step.
- **Custom** — write or paste lyrics and a style. Each field has a **Refine** button, and **Refine everything** takes an optional direction such as "make the chorus punchier".
- **Quick draft** — 8 diffusion steps instead of 32: about a minute, rougher sound. Use it to audition a prompt.
- The Claude/Codex switch at the top picks the refiner. The choice is saved to the config.
- Library: double-click or hover a cover to play. In the detail panel, **Remix** keeps the prompt and uses a new seed. **Reuse prompt** loads the song into Custom.
- Songs generate one at a time; anything else queues. Cancel with the ✕ on a running song.

## Use the CLI

```bash
canto generate --idea "a folk lullaby for a sleepy cat" --draft
canto generate --title "Night Drive" --style "synthwave, male vocal, 110 BPM" --lyrics-file song.txt
canto refine --idea "a stomping bluegrass song about a runaway tractor" --with codex --json
canto refine --style "rock" --lyrics-file song.txt --only lyrics --instruction "tighter rhymes"
canto list --json
canto show <id> | canto favorite <id> | canto rename <id> "New title" | canto delete <id> | canto export-mp3 <id>
```

`generate --json` streams JSONL: `refining`, `refined`, `started`, then `progress` events (`stage`, `fraction`, `detail`), and finally `done` with the song. Errors are `{"error": "..."}` with exit code 1.

## Where things live

| What | Where |
|---|---|
| Config | `~/.config/canto/config.json` (override with `CANTO_CONFIG`); missing means defaults |
| Songs | `library_dir`, default `/Volumes/Swift/local-inference/yue2/outputs/<id>/` |
| Canto's notes on a song | `<id>/canto.json` (title, favorite, status, the request) |
| Engine log | `<id>/canto.log` |
| Deleted songs | `<library_dir>/.trash/` |

Canto reads songs that other tools wrote into the same folder, and never edits YuE2's own files.

## Configuration

```json
{
  "yue_project": "/Volumes/Swift/local-inference/yue2/mlx-Yue",
  "library_dir": "/Volumes/Swift/local-inference/yue2/outputs",
  "precision": "8bit",
  "memory_budget_gib": 16.0,
  "vae_core_frames": 128,
  "refiner": "claude",
  "refiner_model": null
}
```

## When generation fails

YuE2 stops itself if macOS starts swapping, to protect the GPU. Canto reports this as "Not enough free memory" and names the largest process. Quit or unload whatever is holding memory (for example a loaded Ollama model), then press **Remix** or **Reuse prompt**. The full engine output is in the song's `canto.log`.

## Develop

```bash
make test
make run      # debug build of the app
```

Adapters are the seams: `Engine` (`crates/canto-core/src/engine.rs`) for music models and `Refiner` (`refine.rs`) for agent CLIs. A new model or CLI is a new adapter, with no changes to the app.
