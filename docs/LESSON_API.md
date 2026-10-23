# Lesson API

Additive Rhai surface for narrated physics lessons. Everything that already worked
(`runwee render`, `voiceover`, `voiceover_audio`, `arrow`, `vector`, `field_lines`, `axes`, …)
still works the same way. `high` / `1080p` is still 1920×1080 at 60 fps.

Lesson videos are rendered headlessly:

```text
runwee render lesson.rhai -o lesson.mp4 -q 1080p30
runwee render lesson.rhai -o lesson.mp4 -q 1080p30 --audio-json audio.json
```

`-q 1080p30` (aliases `fhd30`, `lesson`) is 1920×1080 at 30 fps and wins over a `quality(...)`
call inside the script. The default quality when the flag is omitted is still 2k. ffmpeg muxes
the scene's audio track into the mp4 (AAC, 192 kbps). `--no-audio` skips that.

Asset paths are tried in this order: as given (so a path relative to the process working
directory still wins), then next to `audio.json` when the path came from that file, then next
to the script (`RUNWEE_SCRIPT_DIR`, set by `runwee render`). The Lessons renderer runs with the
working directory set to the lesson folder, so `audio/audio_1.mp3` resolves either way.

## Timing

`lesson_step` and `--audio-json` take each clip's duration from the decoder (native WAV, ffmpeg
for mp3). That is the same sample length that is muxed, so a step that does not overrun its
audio ends when the clips end.

`audio_duration(path)` prefers `ffprobe` (`format=duration`) and falls back to the decoder. Use
`info.duration` inside `lesson_step` when the picture has to match the muxed audio; probe and
decode can differ by a few milliseconds, and the video length is also rounded to a frame
(1/30 s at `1080p30`).

## `lesson_step`

```rhai
quality("1080p30");
theme("lesson");          // warm paper; any existing theme name still works
captions("karaoke");

lesson_step("intro_1", "Dear students, …", ["audio/audio_intro_1.mp3"], |info| {
    // info.id, info.start, info.duration, info.end
    // info.clips[i].path / .start / .duration / .index
    let head = add(title("Electric charge"));
    play(write(head));
    math_board(vec2(3.6, 0.2), 5.4, 5.6);
    let eq = board_line("math", "q = ne");
    play(write(eq).run_time(0.8));
});
```

`lesson_step(id, clips, |info| { ... })` is the same call without narration (no captions).

Each call after the first starts a new scene and copies the current theme, background, and
caption style. There is no cross-fade: a transition would overlap the scenes and make the
video shorter than the sum of the clips. The closure runs, then the cursor is moved forward
to `info.end` if the animations finished early. If they run past the audio, RunWee prints a
warning and keeps the longer timeline — fit the motion inside `info.duration`.

Clips in one step are placed back to back. Several clips on a step are normal; the step
length is their sum.

`next_scene` is unchanged (it still does not copy the theme). Prefer `lesson_step` when the
next scene should keep the lesson look.

## Math board

`math_board(center, width, height)` adds a card and starts a top-to-bottom cursor inside it.
`board_line(kind, content)` returns a **registered** (hidden) object; reveal it with
`play(write(line))` or `show(line)`.

| kind | drawing |
| --- | --- |
| `title` | bold text |
| `text` (default) | body text |
| `math`, `tex`, `eq` | TeX |
| `highlight`, `hl` | TeX with a gold rule |

Call `math_board` again on each step. Rows that are wider than the card are scaled down.
The card is a dark neutral and the type is white, so light themes (including `lesson`) remap
them to ink-on-paper and dark themes leave them as a light type on a raised panel.

## Diagram primitives

These are the ones lesson scripts reach for. The rest of the engine (shapes, graphs, circuits,
`ode`, …) is listed by `runwee api`.

| call | meaning |
| --- | --- |
| `arrow(a, b)`, `vector(v)`, `dashed_line` | bare geometry |
| `vector_arrow(origin, vec, "F", color)` | labelled vector (same as `force_arrow`) |
| `point_charge(p, q)` | ⊕ / ⊖ bead; sign follows `q` |
| `field_lines([[x, y, q], ...])` | integrated field lines across the frame |
| `field_arrows(charges, x0, x1, y0, y1, cols, rows, color)` | lattice of E arrows, length ~ log\|E\| |
| `equipotentials(charges, levels)` | contour lines |
| `axes([x0, x1, step], [y0, y1, step]).size(w, h).build().plot(\|x\| ...)` | graphs |

Palette constants (still remapped by light themes): `LESSON_POS`, `LESSON_NEG`, `LESSON_GOLD`,
`LESSON_METAL`, `LESSON_ACCENT`, `LESSON_INK`, `LESSON_PAPER`.

## `--audio-json`

For a script that never calls `lesson_step`, `voiceover_audio`, or `sound_file`. If the script
already scheduled audio, the file is ignored and a note is printed.

```json
{
  "steps": [
    { "id": "intro_1", "clips": ["audio/audio_intro_1.mp3"] },
    { "id": "1", "clips": ["audio/audio_1.mp3", "audio/audio_1b.mp3"] }
  ]
}
```

Also accepted: a bare array of step objects, a map `{ "intro_1": ["audio/a.mp3"] }`,
`{ "clips": ["audio/a.mp3"] }`, or a flat array of paths (one timeline). A clip may be a
string or `{ "file": "audio/a.mp3" }` (`path` and `src` work too).

- One scene in the script: every clip is concatenated from t = 0 and the scene is extended
  to that duration. Objects added with `add` stay up, so a short animation holds on the
  last frame through the rest of the narration.
- One scene per step (same count, in the same order): each step's clips start at t = 0 of
  that scene, and that scene is extended to its own clips.
- Any other pairing is zipped in order; extra steps are appended on the last scene.

The Lessons workflow passes `--audio-json audio.json` whenever that file sits next to
`lesson.rhai`. Scripts that already embed audio are left alone.

## Quality bar

A lesson should be clearer than the browser canvas: one palette, eased motion (`rate("smooth")`
or the default), and TeX on the board instead of stroked debug text. Keep each step inside its
audio. Do not change Rust; if something cannot be drawn with this API, describe it in
`runwee-suggestions/batch-NN.md` on the Lessons branch.
