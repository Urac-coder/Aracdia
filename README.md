# Aracdia

Voxel RPG inspired by Minecraft, built on the **Luanti** (ex-Minetest) engine,
shipped through a **custom cross-platform launcher** built with **Tauri 2**.

> Status: bootstrap. The launcher skeleton is in place. Game content and server
> are stubs.

## Why not just mod Minecraft?

Aracdia targets a commercial release. Mojang's EULA forbids selling Minecraft
mods or distributing the Minecraft client, so we build on the
[Luanti](https://www.luanti.org/) engine (LGPL — explicitly allows commercial
games on top of it).

## Repository layout

```
.
├── launcher/   # Tauri 2 desktop launcher (Rust + React + TypeScript)
├── game/       # Luanti "game" with our signature mods (Lua)
├── server/     # Dedicated server config + Docker setup
├── shared/     # Cross-language schemas / constants
├── docs/       # Architecture decisions, design docs
└── .github/    # CI workflows
```

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the design rationale.

## Quickstart — launcher (macOS / Windows / Linux)

Prerequisites:

- **Node 22+** and **npm 10+**
- **Rust stable** via [rustup](https://rustup.rs/)
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Linux: `libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev patchelf`
- Windows: Microsoft Edge WebView2 runtime (preinstalled on Win11)

```bash
cd launcher
npm install
npm run tauri dev      # launches the dev app with hot-reload
npm run tauri build    # produces a signed bundle in src-tauri/target/release/bundle
```

## Roadmap

- [x] **Step 0** — Tauri 2 project scaffold + cross-OS CI
- [x] **Step 1** — Offline login + home UI
- [ ] **Step 2** — Cross-platform paths + persisted settings
- [ ] **Step 3** — Engine binary download (with progress + SHA-256 verification)
- [ ] **Step 4** — Game content download + extraction
- [ ] **Step 5** — Engine spawn (subprocess) with proper args
- [ ] **Step 6** — Launcher self-update (Tauri Updater + Ed25519 signing)
- [ ] **Step 7** — Polish (settings, RAM, server picker, error states)
- [ ] **Step 8** — Game `mods/` (Lua): weight inventory, MOBA-like spells, custom UI
- [ ] **Step 9** — Dedicated server Docker stack

## License

To be decided. The launcher and game content are project code; the underlying
Luanti engine is LGPL.
