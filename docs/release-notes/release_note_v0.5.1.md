# Release v0.5.1 — Good Morning

The pet now yawns when it wakes. Leave it alone for two minutes and it lies down and
breathes; move the cursor onto it, and instead of snapping awake it stretches into a
slow, wide yawn before settling back.

## ✨ New

- **It yawns when it wakes.** Waking from sleep is now the biggest thing the face
  does: the mouth opens slowly from the resting grin to the widest it ever gets — a
  full second of jaw drop, then a held peak with a small shiver — and closes again
  just as slowly. The eyes come open already narrowed and only open fully once the
  mouth has shut, so it reads as one lazy arc rather than a face that wakes and only
  then decides to yawn.
- **Slow, not snappy.** The whole thing runs for three and a half seconds, and it
  starts the instant the pet wakes — there is no beat where it sits awake and normal
  first.

## 🔧 Under the hood

- `yawn` is the first state this fork *adds* to the engine, so it lives in both twins
  — the TypeScript reference and the Rust port — and the golden parity suite grew
  from 142 to **148 cases**.
- The mouth it opens is the one from 0.5.0: same closed path, same fill and stroke,
  same anchor between the eyes. Only its numbers change, so there is no new geometry
  and no painter change.
- The yawn was judged on all eight silhouettes at 180 and 512 px, and its opening was
  tuned against measured frames rather than by eye alone.

## 📦 Downloads

- **`Dot_0.5.1_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- Windows only, as before.
- Nothing to configure: the yawn fires on wake. If the mouth is switched off in
  Settings, the pet still yawns — with no mouth, as every other state does.
- Exclusive-fullscreen games cover it, as they do any window.