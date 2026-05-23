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
- [x] **Step 2** — Persisted settings (memory, server, install dir, manifest URL)
- [x] **Step 3a** — Engine pipeline: download + SHA-256 verification + zip extract + auto-install at "Play"
- [x] **Step 3b** — `aracdia-engine` repo created at [`Urac-coder/aracdia-engine`](https://github.com/Urac-coder/aracdia-engine) with a CI release workflow producing zips for macOS arm64, Linux x64 and Windows x64 (see [`docs/CREATE_ARACDIA_ENGINE.md`](docs/CREATE_ARACDIA_ENGINE.md))
- [x] **Step 5** — Engine spawn (subprocess) with stdout/stderr capture, rolling log file, macOS quarantine handling, single-process invariant, "Quitter le jeu" UI
- [x] **Phase A.1** — Minimum Aracdia game content: `game.conf`, menu assets, `aracdia_core` mod with 7 nodes (dirt / grass / stone / sand / water / wood / leaves), mapgen aliases, 3 biomes, placeholder textures. Verified to boot a headless server on `--gameid aracdia` with no errors.
- [x] **Phase A.3 (a)** — `game/` is bundled as a Tauri resource and auto-deployed to `<luanti_user>/games/aracdia/` before each spawn (idempotent, SHA-256 signature check skips redundant copies). Engine receives `--gameid aracdia` so the user lands directly in Aracdia, never in the bare Luanti menu.
- [x] **Phase A.3 (b) / Step 4** — Game content is published as versioned `aracdia-game-vX.Y.Z` releases on the monorepo via [`.github/workflows/game-release.yml`](.github/workflows/game-release.yml). The launcher fetches the latest, verifies the SHA-256 and atomically swaps it into Luanti's user games dir, taking precedence over the bundled fallback. First release: [`game-v0.1.0`](https://github.com/Urac-coder/Aracdia/releases/tag/game-v0.1.0).
- [x] **Phase B.1** — The launcher now spawns a Luanti **server** locally and the JOUER button connects the **client** to it (`--address 127.0.0.1 --go`). The Luanti vanilla menu is never shown; players can't create worlds nor join unknown servers. Default bind is `0.0.0.0`, so LAN friends can join immediately, and Internet friends can join via port forwarding. The launcher panel shows status / port / PID and the future-VPS migration is just a toggle (server address override in settings).
- [ ] **Step 6** — Launcher self-update (Tauri Updater + Ed25519 signing)
- [ ] **Step 7** — Polish (settings, error states, telemetry)
- [ ] **Step 8** — Game `mods/` (Lua): weight inventory, MOBA-like spells, custom UI
- [ ] **Step 9** — Dedicated server Docker stack

## License

To be decided. The launcher and game content are project code; the underlying
Luanti engine is LGPL.
