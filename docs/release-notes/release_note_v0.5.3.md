# Release v0.5.3 — Machine Mood

The pet now wears how hard your PC is working.

## ✨ New

- **A mood from your machine.** It reads CPU load and battery — offline, in the same
  process, with no extra thread and no network — and rests in one of four faces: your
  own expression while the box is calm, **Attentive** when it is busy (CPU ≥ 45 %),
  **Scared** and wide-eyed when it is pinned (≥ 85 %), and **Sleepy** on a low
  battery (≤ 20 %).
- **A switch, on by default.** Settings → **Machine mood**. Off, the pet wears exactly
  the expression you chose and never changes its face behind your back.

## 🔧 Under the hood

- CPU load is sampled once a second and averaged with an EMA, and the mapping carries
  hysteresis, so a machine sitting on a threshold cannot flicker the face.
- It only ever changes the *resting* face: an emote still wears its own, and your
  expression comes back the moment the machine goes quiet.
- No engine, painter or golden change — it reuses expressions the catalogue already
  had, so the 148-case parity suite is untouched.

## 📦 Downloads

- **`Dot_0.5.3_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- Windows only, as before.
- A desktop that reports no battery is never drowsy; the mood then follows CPU alone.
- Exclusive-fullscreen games cover it, as they do any window.
