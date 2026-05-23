# Aracdia — architecture

## Goals

- Voxel RPG with **weight-based inventory** (no item stacks)
- **MOBA-like combat**: 4–5 spells per class, skillshots, cooldowns
- **Custom UI**, distinct from Minecraft's visual identity
- Multiplayer **50–200 concurrent players**
- Cross-platform desktop: **macOS (dev), Windows, Linux**
- **Commercial-ready** (no Minecraft IP entanglement)

## Stack overview

| Layer | Tech | Why |
|---|---|---|
| Game engine | Luanti (ex-Minetest), C++, **LGPL** | Mature voxel engine, native multi, commercial OK, mods in Lua |
| Game content | Lua mods in `game/` | Fast iteration on inventory, combat, UI |
| Launcher UI | React 19 + TypeScript + Tailwind v4 + Vite 7 | Modern, fast iteration, ecosystem |
| Launcher backend | Rust 1.95 + Tauri 2 | Tiny binaries (~10 MB), native multi-OS, secure IPC |
| Update channel | GitHub Releases (manifest + assets) | Free, simple, signed |
| Auth | Offline (UUID v4 local) | Step 1 — server-side auth comes later |
| Dedicated server | Luanti server in Docker | Standard deployment for 50–200 CCU |

## Why Tauri rather than Electron / JavaFX

- **Native multi-OS** (macOS arm64 + x64, Windows x64, Linux x64) without bundling Chromium
- **~10 MB binaries** vs ~150 MB for Electron — important for a launcher
- **Strong sandbox + capability model** — safer for an app that downloads and runs other binaries
- **Rust backend** — perfect language for the file IO + process spawning + integrity checks the launcher does
- A Java launcher would make sense if we were targeting Java Edition Minecraft, but we're not

## Launcher data flow (target)

```
┌──────────────────────────────────────────────────────────────────┐
│                         Aracdia Launcher                         │
│                                                                  │
│   ┌────────────┐       Tauri IPC      ┌─────────────────────┐    │
│   │  React UI  │  <───────────────>   │   Rust commands     │    │
│   │ (TS / TW)  │                      │  profile / engine / │    │
│   │            │                      │  game / launch      │    │
│   └────────────┘                      └──────────┬──────────┘    │
│                                                  │               │
│                       ┌──────────────────────────┼─────────┐     │
│                       ▼                          ▼         ▼     │
│              ~/Library/Application       Engine binary    Game   │
│              Support/Aracdia/            (Aracdia Engine, content│
│              Launcher/                    forked Luanti)  zip    │
│                ├ profile.json                                    │
│                ├ engines/                                        │
│                ├ games/                                          │
│                └ logs/                                           │
└──────────────────────────────────────────────────────────────────┘
                  │ spawn (--gameid aracdia --go)
                  ▼
            ┌──────────────────┐
            │ Aracdia Engine   │  ← runs the actual game
            │ (Luanti fork)    │
            └──────────────────┘
                   │ network
                   ▼
            ┌──────────────────┐
            │ Aracdia Server   │  ← dedicated server, Docker
            │ (Luanti server)  │
            └──────────────────┘
```

## Directory conventions per OS

Resolved by the `directories` crate (`com.Aracdia.Launcher`):

- macOS: `~/Library/Application Support/com.Aracdia.Launcher/`
- Windows: `%APPDATA%\Aracdia\Launcher\`
- Linux: `~/.local/share/aracdia-launcher/`

Inside that directory:

- `profile.json` — current player profile
- `settings.json` — launcher preferences (RAM, server, window state) — *step 7*
- `engines/<version>/` — extracted engine builds — *step 3*
- `games/<name>-<version>/` — extracted game content — *step 4*
- `logs/launcher.log` — rolling log file — *step 7*
- `cache/` — temporary downloads

## Engine strategy

We **fork Luanti** as `aracdia-engine` (LGPL — fork is allowed and we will
publish our source for the engine portion). The launcher downloads our
engine builds from the `aracdia-engine` repo's GitHub Releases, not the
upstream Luanti releases. This gives us full branding control and the ability
to ship custom protocol/feature changes if needed.

Until the fork is set up, the launcher can temporarily target upstream Luanti
releases for testing.

## Versioning & manifest

The launcher fetches a JSON manifest from a known URL (e.g.
`https://github.com/aracdia/launcher/releases/latest/download/manifest.json`):

```json
{
  "launcherVersion": "0.1.0",
  "engine": {
    "version": "5.10.0-aracdia.1",
    "assets": {
      "aarch64-apple-darwin":   { "url": "...", "sha256": "..." },
      "x86_64-apple-darwin":    { "url": "...", "sha256": "..." },
      "x86_64-pc-windows-msvc": { "url": "...", "sha256": "..." },
      "x86_64-unknown-linux-gnu": { "url": "...", "sha256": "..." }
    }
  },
  "game": {
    "version": "0.1.0",
    "url": "...",
    "sha256": "..."
  },
  "server": {
    "address": "play.aracdia.example",
    "port": 30000
  }
}
```

## Decisions log

| # | Decision | Rationale |
|---|---|---|
| 1 | Use Luanti, not Minecraft modding | EULA blocks commercial Minecraft mods |
| 2 | Tauri 2 launcher (Rust + React) | Tiny native binaries, cross-OS, modern UX |
| 3 | Offline login first | Unblocks development; auth server can be added later |
| 4 | GitHub Releases for asset hosting | Free, simple, sufficient until thousands of players |
| 5 | Fork Luanti as `aracdia-engine` | Branding control, full ownership of release flow |
| 6 | Monorepo | Atomic changes across launcher/game/server, single CI |
