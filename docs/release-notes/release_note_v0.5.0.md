# Release v0.5.0 — A Face

The pet has a mouth. It smiles when it is idle, frowns when it is cross, opens wide
when it is startled, and it can be switched off if you liked it better as two eyes on
a ball. It also reacts to being picked up and put down, and falls asleep if you leave
it alone.

## ✨ New

- **A mouth.** One closed path, filled and stroked, so a closed mouth is a rounded
  line and an open one is a D rather than a wedge — and every number in it
  interpolates, so an expression can part the mouth without swapping shapes.
  All sixteen expressions carry their own: frowns for **Angry** and **Sad**, a round
  "o" for **Surprised** and **Scared**, the widest grin for **Hilarious**, a flat line
  for **Bored**. The emoting states have theirs too, and `thinking`, `alert`,
  `exclaim` and `sleep` have no face at all, as before.
- **It follows the head.** The mouth sits where the engine actually put the eyes, and
  it is canted to their line. Without that, a pet looking straight at you with its
  head cocked wore a level mouth underneath, which reads as a smirk rather than as a
  tilted head.
- **Mouth: On / Off.** A new row in Settings, under **Expression**. Off is the
  original pet — eyes only, in every state, with the thumbnails in step.
- **It reacts when you put it down.** Drop it after a drag and it looks startled for a
  moment; a click that did not move it stays a click.
- **It sleeps when you leave it alone.** Two quiet minutes and it lies down and
  breathes; move the cursor onto it or click, and it wakes.

## 🔧 Under the hood

- The mouth is data before it is a branch: width, bow, lip thickness and opening ride
  on the pose and interpolate like the eyes, so it needed no new kind of element and
  no painter branch. `docs/adding-the-mouth.md` is the port guide.
- The mouth was chosen against real frames before it was coded — five candidates, all
  eight silhouettes, three sizes, three inks.
- The engine still exists twice, joined by the golden suite: **142 parity cases**,
  now including the mouth.
- Click-through is now verified rather than assumed: a scripted probe drives the real
  cursor and reads the window's `WS_EX_TRANSPARENT` bit.
- Still one native process, ~1 % of a core and ~15 MB at rest, no WebView2 for the
  pet.

## 📦 Downloads

- **`Dot_0.5.0_x64-setup.exe`** — installer (NSIS). Recommended.
- **`dot.exe`** — portable, no install.

## ⚠️ Notes

- Windows only, as before.
- The mouth is on by default. Turning it off removes it from every state and from the
  Settings thumbnails; your `config.json` keeps the choice.
- Exclusive-fullscreen games cover it, as they do any window.