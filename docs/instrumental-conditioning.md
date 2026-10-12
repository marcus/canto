# Explicit instrumental conditioning

`SongRequest.instrumental` defaults to false and is omitted from serialized JSON when false, preserving old requests and songs. Ordinary requests still require nonempty lyrics. Explicit instrumental requests require empty or whitespace-only lyrics; section markers and sung text are rejected. A nonempty style remains required. This is a request for model conditioning, not a verified property of the resulting audio.

The CLI exposes `generate --instrumental --style "piano, jazz"`, optionally combined with `--max-duration-seconds N`, seed, mode and draft. It rejects `--idea` and `--refine` because the current refiner prompt produces sung lyrics; it never discards supplied text or rewrites refiner behavior. Validation failures use the existing JSON error and exit 1 path before claiming a song directory or invoking a refiner.

The mlx-Yue adapter translates the request into an empty native lyric string and prepends an `instrumental` style tag unless that comma-delimited tag already exists. No native `instrumental` field is invented. Mode, steps, sampling defaults, duration limits and memory/swap guards retain their existing behavior. Instrumental intent does not override a contradictory vocal style tag; callers should use an instrumental arrangement description.

Canto's own `canto.json` stores the original request with `instrumental: true`. Song JSON and `Song.request()` carry it forward into remixes, including failed or incomplete songs. Native engine files remain read-only, and an old or externally generated song does not infer intent from an empty lyric string or an instrumental style tag. A caller adding text to a remix must also set `instrumental = false`; otherwise core validation refuses the contradictory request. The desktop Remix path preserves the flag; desktop Custom and Reuse prompt currently create ordinary requests and need nonempty lyrics. Explicit opt-in creation is intentionally available through CLI/core only for this narrow slice.

## Native compatibility evidence

Read-only inspection of the configured installed mlx-Yue checkout confirms:

- `skills/yue2-music/references/optimization.md` recommends trying empty lyrics, explicit instrumental style and an Ins score, and states there is no hard zero-vocal switch. The instrumental flag supplies empty lyrics and style conditioning; [supplied-score creation](supplied-scores.md) can also provide a native Ins score.
- `vendor/yue/src/yue2/protocol.py`, `SongRequest.__post_init__`, requires strings but permits an empty lyric string. `text()` emits the empty `[Lyrics]` section without adding words.
- `vendor/yue/src/yue2/pipeline.py`, `_request`, rejects `None` for lyrics but accepts an empty string. The local MLX pipeline delegates request normalization to that shared path.
- A protocol-only Python check constructs and serializes empty-lyric native requests for full, melody and off modes successfully, without loading models or running GPU generation.

Fifteen focused CLI/core tests pass. The distinct private release binary is built with `cargo build --release -p canto-cli --target-dir target/instrumental`; it leaves the existing radio binary intact. Structured CLI journey evidence is saved in [instrumental conditioning evidence](evidence/instrumental-conditioning.json). CLI/core tests and a temporary substitute engine prove request parsing, refusals, adapter translation, metadata and remix inheritance. They do not prove instrumental audio quality or absence of vocals. The serial radio coordinator owns the first native trial and must inspect its normalized request and resolved configuration, then independently audit its audio. No native runtime/song edits or new GPU rendering are part of this implementation.

## Observed native trial

The serial radio coordinator rendered the approved instrumental candidate with an empty native lyric string, full mode, an explicit instrumental style, a 110-second requested cap and the unchanged 16 GiB guard. The primary request/config and Canto metadata retained that conditioning. Its audio audit found 131 recognized words and four singing spans, so the track was rejected. This verifies the request path while demonstrating that the option cannot promise absent vocals. Primary-field provenance is in [native trial evidence](evidence/instrumental-native-proof.json). The off-mode attempt stopped at the memory/swap guard and produced no vocal result to assess.
