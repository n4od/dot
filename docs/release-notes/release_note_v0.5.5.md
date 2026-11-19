# Release v0.5.5 — Your Pace

The two-minute nap was too short a leash, and it was the only number in Dot you
could not change. Now you can.

## ✨ New

- **Sleep after: your call.** Settings has a new row — **Sleep after**: 5, 15, 30 or
  60 minutes, or **Never**. The default is now **15 minutes** instead of a fixed two.
- **The clock is unchanged.** It still measures *no keyboard or mouse input anywhere*,
  so working at the PC keeps the pet awake; it only lies down when you have actually
  stepped away.
- **Never means awake.** Choosing **Never** brings a pet that was already asleep
  straight back, rather than leaving it down until you happen to touch it.

## 🔧 Under the hood

- The delay is read from `config.json` (`sleepMinutes`) and applied in place, like
  every other setting — no restart.
- "Never" is represented as a missing duration rather than a magic number, and the
  sleep decision stayed a pure function, so both cases are unit-tested.
- No engine, painter or golden change — the 148-case parity suite is untouched.

## 📦 Downloads

- **`Dot_0.5.5_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- Windows only, as before.
- A config written before this release has no `sleepMinutes` key, so it opens on the
  new 15-minute default.
- With **Never** the pet never sleeps on its own, so the wake-up yawn never plays —
  it is still there the moment you turn sleep back on.
- Exclusive-fullscreen games cover it, as they do any window.