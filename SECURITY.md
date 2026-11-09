# Security Policy

## Reporting a vulnerability

Please report privately rather than in a public issue — use GitHub's
**Security → Report a vulnerability** if it is enabled on this repository, or
email **naodtiger@gmail.com**.

Include:

- what you found and where (a file, a command, or a release),
- how to reproduce it,
- the version you tested (the installer name, or the relevant `CHANGELOG` entry).

I will acknowledge as soon as I can, and I would ask you to keep it private until
a fix is released.

## What is in scope

Dot is a desktop app with a small surface:

- the settings window (a Tauri / WebView2 document),
- reading and writing `config.json` (`src-tauri/src/lib.rs`),
- the native pet window and its Win32 message handling (`src-tauri/src/pet.rs`),
- the release installers and the portable executable.

The animation engine under `src/bot/` is a copy of
[`jeremy-prt/bloub`](https://github.com/jeremy-prt/bloub) (MIT); upstream issues
belong there, though a fix may need to land in both.

## Supported versions

Only the most recent release is supported.
