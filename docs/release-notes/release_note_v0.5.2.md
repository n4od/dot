# Release v0.5.2 — Stay Awake

A small fix with a big quality-of-life payoff: the pet no longer falls asleep while
you are actually working.

## 🐛 Fixed

- **It stays awake while you use the PC.** The sleep timer used to count only the
  cursor sitting *on the pet*, so typing, scrolling or clicking anywhere else left the
  clock running — and it lay down every two minutes no matter how busy you were. It now
  reads the system's last-input time (the same clock the screensaver uses), so it
  sleeps only after two minutes with no keyboard *and* no mouse input anywhere. Work
  beside it all day and it stays up; leave the desk and it naps; come back and move the
  pointer onto it and it wakes with the yawn from 0.5.1.

## 🔧 Under the hood

- The decision is a pure function, so it is covered by unit tests instead of needing a
  real desktop — including the case of a pointer parked on the pet, which must not
  fall into a sleep → yawn → sleep loop.
- No hooks and no extra polling: `GetLastInputInfo` is a single cheap read per frame,
  the same class of call the window loop already makes.

## 📦 Downloads

- **`Dot_0.5.2_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- Windows only, as before.
- Nothing to configure: the wake-up yawn and the sleep timer are automatic.
- Exclusive-fullscreen games cover it, as they do any window.
