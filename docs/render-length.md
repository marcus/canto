# Optional render length

`SongRequest.max_duration_seconds` is an optional whole-second limit from 1 to 360. Core validation rejects zero and larger values. The CLI exposes `generate --max-duration-seconds N` with normal human or JSONL output. It validates the bound before prompt refinement and validates the full request before claiming a song directory. Omitted limits serialize exactly as before and do not set model sampling overrides.

`MlxYue` is the only translation boundary: a bounded request adds `semantic_sampling.max_tokens = N × 25` and `min_tokens = min(200, max_tokens)` to the temporary mlx-Yue request. Existing acoustic steps, ABC planning and memory/swap guards are unchanged. This limits generated semantic frames; it does not constrain symbolic planning time or guarantee that a phrase ends naturally before truncation.

Canto's `canto.json` retains the original request and limit. `Library` exposes it as `Song.max_duration_seconds`, and `Song.request()` carries it into a remix. Native YuE2 request and result files remain read-only. Legacy request metadata and song JSON lacking the optional field deserialize to `None`. Canto never infers a limit from an old song's actual length or engine token configuration.

## Verified adapter contract

The configured mlx-Yue executable's Python environment resolves `lyra` to the installed checkout's `src/lyra` and `yue2` to `vendor/yue/src/yue2`. The VAE `models/vae/config.json` declares `sample_rate: 48000` and `downsampling_ratio: 1920`; its decoder strides multiply to 1920. `src/lyra/nar.py` requires one acoustic noise/latent frame per semantic codec token. `src/lyra/pipeline.py` checks that saved audio has exactly `1920 × frames − 64` samples, and `src/lyra/vae.py` implements that output geometry. These give 25 frames per second. For example, the existing `crooked-cassette` result has 9,000 semantic-token sampling ceiling and 359.9986667 seconds of decoded audio.

A 90-second request has at most 2,250 semantic tokens, a 1-second request at most 25, and the maximum 360-second request at most 9,000. The current decoder is slightly shorter than the corresponding whole-second ceiling. Other decoder releases may add boundary padding; the verification allowance is one codec frame, 40 ms. This is a maximum, not a target duration or an audio postprocessing trim.

## Validation

Core and CLI tests cover omission/old JSON, request round trips, 1/90/360-second bounds, rejected out-of-range values, adapter token mapping and minimum-token clamping, library loading and remix retention, and preservation of native engine files. A CLI-only release binary is built for isolated callers; no installed application or global CLI needs replacement. A real bounded render must be audited by the serial generation coordinator before claiming observed output duration or runtime improvement. No GPU generation is part of the unit checks.
