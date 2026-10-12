# Supplied ABC scores

`SongRequest.abc` optionally contains a supplied native ABC score as text. `None` is the default and is omitted from JSON, preserving ordinary planner behavior and old metadata. A supplied score must contain non-whitespace text and use full or melody mode. Off mode cannot consume a score. Core validation enforces these rules; the CLI and model adapter both apply them before their effects.

`canto generate --abc-file theme.abc --style "piano, jazz" --instrumental --mode full --max-duration-seconds 90 --json` reads the UTF-8 file once before refinement or directory creation. Missing, unreadable, blank and off-mode score inputs fail through the existing JSON error path. The request stores text, never the file path. Later changes or removal of the source file do not change a stored request or remix. Existing sung requests can also supply a score while keeping their lyrics.

The mlx-Yue adapter passes the exact text in the native `abc` field; it does not generate an `abc_path`, rewrite a score or alter mode, steps, sampling or memory/swap guards. The native pipeline uses supplied score text in its planner prefix rather than automatically planning a new score. Canto does not implement a second ABC parser or infer musical validity from a nonblank string. Inspect a score with the native ABC helper before generating; semantic/context limits and model adherence still apply.

Canto's `canto.json`, song JSON and `Song.request()` retain the score text. Renaming or remixing does not edit native engine files. A caller changing a remix to off mode must explicitly remove its score (`abc = None`). Legacy/native-only songs do not infer supplied-score intent from native artifacts. Explicit score creation is CLI/core only in this narrow slice. Desktop Remix preserves the score, while Custom and Reuse prompt create ordinary requests without a supplied score.

## Native format and instrumental conditioning

Installed guidance in `skills/yue2-music/references/abc-editing.md` specifies two monophonic parts. `Ins` carries an instrumental melody or solo, rather than piano chord voicings. Quoted harmony symbols belong in `Vocal`, including when that part rests. Full-mode supplied scores can retain those chord symbols. Melody mode needs a chord-free score; use the native strip-chords helper rather than expecting Canto to rewrite it.

```abc
X:1
T:
M:4/4
L:1/32
Q:1/4=88
V: Vocal clef=treble name="Vocal Melody" snm="Vocal"
V: Ins clef=treble name="Ins Melody" snm="Inst."
K:G
% interlude
V: Vocal
"G"z32|"D7"z32|
V: Ins
G8B8d8B8|A8F8D16|
```

This two-bar example demonstrates format, not a complete composition. Both parts cover the same bar grid. Explicit instrumental style and empty lyric conditioning plus an Ins melody are requests, not a zero-vocal switch or a guarantee of audio adherence. Native empty-lyric full-mode generation has already produced vocal material; supplied-score audio requires its own audit. The off-mode attempt stopped at the unchanged memory/swap guard, so no vocal result was observed for that attempt.

## Verification boundaries

Nineteen focused core/CLI tests cover optional-field omission, score text round trips, blank/off refusals, exact native adapter passthrough, metadata/native-file preservation, and remix retention/removal. The separate private CLI release binary is built in `target/score`; the approved `target/instrumental/release/canto` is preserved. An affected desktop source check uses the existing separate target without running or installing the app.

The installed native protocol accepts score text in full/melody and rejects blank/off requests. Its standard-library ABC inspector accepts the two-bar fixture and records Vocal rests, Ins notes and matching bar grids in [native inspection](evidence/supplied-score-native-inspection.json). Structured [CLI evidence](evidence/supplied-score-cli.json) records the exact private binary hash and guard argument. CLI checks against a temporary substitute engine prove parsing, pre-effect errors, text snapshotting and provenance, not actual generated audio. The serial radio coordinator owns the native supplied-score trial, source/config verification and independent audio audit. No GPU generation or native runtime/song edits are part of this implementation.
