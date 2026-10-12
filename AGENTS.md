# AGENTS.md — working on Canto

Canto is a local song studio over YuE2: a Rust workspace with a core library, an agent CLI (`canto`), and a GPUI desktop app (`canto-app`). Read `README.md` for what it does and where data lives.

## Rules

- **Core owns behaviour.** Anything the app can do goes through `canto_core::Studio` and must also be reachable from the `canto` CLI with `--json`. The app holds view state only.
- **Adapters at the seams.** Music models implement `Engine`; agent CLIs implement `Refiner`. Don't call `mlx-yue`, `claude` or `codex` from anywhere else.
- **Never edit YuE2's files** in a song directory. Canto's own state lives in `canto.json` and `canto.log`.
- **Builds need Xcode's Metal compiler.** `.cargo/config.toml` sets `DEVELOPER_DIR`; keep it.
- The refiner prompt in `refine.rs` encodes what YuE2 responds to (tag-style English prompts, section-tagged lyrics, a repeated chorus). Change it deliberately and test with a real generation.
- Generation needs about 11 GB free. YuE2 aborts on new swap. Don't weaken that guard to make a run pass.

## Commands

```bash
make test        # unit tests
make run         # debug app
make install     # ~/Applications/Canto.app + ~/.local/bin/canto
canto list --json
```

Work is tracked in `td` in this repo.
