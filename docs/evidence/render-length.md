# Render length verification

Candidate scope: optional `SongRequest.max_duration_seconds` and `canto generate --max-duration-seconds N`, with translation confined to the mlx-Yue adapter. No installed engine/runtime files or existing song directories were modified. No GPU generation was run for this change.

## Local checks

- `cargo test -p canto-core -p canto-cli`: 10 core tests and 1 CLI test passed.
- `cargo build --release -p canto-cli`: passed. The private binary is `/Users/marcus/code/canto-rar-length-oct11/target/release/canto`; no global installation was performed.
- The private binary's `generate --help` exposes the optional whole-second 1–360 limit.
- With a temporary `CANTO_CONFIG` pointing to a nonexistent engine and temporary library, `generate --idea ... --max-duration-seconds 0 --json` and the same command with 361 both exit 1 with `{"error":"max_duration_seconds must be between 1 and 360"}`. Neither creates a library/song directory or invokes a refiner.
- Adapter tests check omission leaves native sampling overrides absent, and 1/7/90/360 seconds map to 25/175/2250/9000 maximum tokens with minimums 25/175/200/200. Library tests check old metadata, bounded request retention through loading/renaming/remix, and byte-for-byte native request file preservation.

## Installed codec inspection

The configured project is `/Volumes/Swift/local-inference/yue2/mlx-Yue`. Its `.venv/bin/mlx-yue` entrypoint uses `lyra.cli`, and Python module resolution points to this checkout's `src/lyra` and `vendor/yue/src/yue2`.

- `models/vae/config.json`: 48,000 Hz audio and 1,920 downsampling ratio, giving exactly 25 codec frames per second.
- `src/lyra/nar.py`, `_chunks_with_supplied_noise`: one noise/latent frame per semantic token.
- `src/lyra/pipeline.py`, `SongResult.save_artifacts`: verifies the decoded sample count is `1920 * frames - 64`.
- `src/lyra/vae.py`, `output_length`: implements the same decoder geometry.
- `vendor/yue/src/yue2/protocol.py`, `Sampling`: accepts request-level sampling overrides; requires `0 <= min_tokens <= max_tokens` and positive maximum; defaults are 200 and 9000.
- Existing `crooked-cassette/result.json` has 359.9986666666667 audio seconds and a 9000 semantic-token ceiling, consistent with that geometry.

## Remaining observation

The serial radio coordinator owns the first bounded render and output audit. Unit and adapter checks do not prove a new rendered duration, musical ending, or runtime improvement. The current decoder's samples fall below the limit; a different decoder's verification allowance is one codec frame (40 ms). ABC planning remains unchanged, and the memory/swap guard is preserved.
