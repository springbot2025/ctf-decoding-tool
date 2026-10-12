# CTF Tool

[English](README.md) | [简体中文](README.zh-CN.md)

A desktop decoding / decryption toolkit for CTF competitions. Rust workspace + Tauri 2 + React. Paste ciphertext, get results instantly — with chained decoding and automatic flag detection.

## Features

- **Encodings (21)**: Base64/32/16/85/58/62, Hex, Binary, Octal, Decimal, URL, HTML entities, Unicode escapes, Morse, Brainfuck, DNA, Bacon, Polybius square, ROT13, Atbash, reverse
- **Classical ciphers**: Caesar (with brute force), Vigenère, Affine, Rail fence, Columnar transposition, Beaufort
- **Modern ciphers**: XOR (with single-byte brute force), AES-128/192/256 (ECB/CBC), RC4, RSA (n + d)
- **Utilities**:
  - Auto-detect — tries every keyless method, ranked by printable ratio + flag-hit bonus
  - Flag finder — custom flag header, full-text highlighting, one-click copy
  - Chained decoding — "use as input" feeds output back for multi-layer encodings

Invalid bytes are dropped silently during decoding (`errors=ignore` semantics).

## Quick start

```bash
cd frontend
npm install
npm run app        # dev mode (Vite HMR + Tauri window)
```

Build a release executable:

```bash
cd frontend
npx tauri build --no-bundle   # output: target/release/ctf-app.exe
# drop --no-bundle for MSI/NSIS installers (requires WiX/NSIS)
```

## Project layout

```
crates/ctf-core/      core decoding library (pure Rust: 33 methods, auto-detect, flag extraction)
crates/ctf-gui/       Tauri shell (FIXME: duplicate of frontend/src-tauri, to be removed)
frontend/             React + Vite frontend
frontend/src-tauri/   Tauri shell (the one actually used, package name ctf-app)
```

## Tests

```bash
cargo test -p ctf-core    # 35 tests (incl. edge-case regressions)
```

## Requirements

- Rust (MSVC toolchain) + Visual Studio 2022 (C++ build tools)
- Node.js 18+
- Note: if the `CC`/`CXX` environment variables point to MinGW gcc, MSVC linking fails with LNK1143 — unset them first
