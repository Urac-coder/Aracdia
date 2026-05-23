# How to create the `aracdia-engine` repo

The Aracdia launcher downloads its game engine from a separate repository
called **`aracdia-engine`** — a fork of [Luanti](https://github.com/luanti-org/luanti)
(formerly Minetest) rebranded as the Aracdia engine. This document is the
checklist to bootstrap that repo so the launcher can install something real.

> **Why a fork rather than upstream?** We need branding control (window
> title, splash, default game id), and the freedom to ship custom engine
> changes later (e.g. tweaks to the network protocol). Luanti is LGPL — we
> are required to publish the source of any modified engine builds, which is
> exactly what `aracdia-engine` does.

## Asset contract expected by the launcher

For each GitHub release, the launcher looks for **two assets per target**:

| File | Content |
|---|---|
| `aracdia-engine-<target>.zip` | Engine binaries + assets, ready to extract into `<install>/engine/` |
| `aracdia-engine-<target>.zip.sha256` | One line, hex SHA-256 of the zip (à la `sha256sum`) |

Supported `<target>` values (must match Rust target triples):

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-msvc` *(optional)*
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu` *(optional)*

The launcher resolves manifests via the GitHub Releases API:

```
https://api.github.com/repos/aracdia/aracdia-engine/releases/latest
```

So **the assets must be attached to a published release** (not just stored as
raw files in the repo).

## Step 1 — Create the repo

1. Create an empty repo `aracdia/aracdia-engine` on GitHub (any visibility,
   public is fine).
2. Locally:
   ```bash
   git clone --depth 1 https://github.com/luanti-org/luanti.git aracdia-engine
   cd aracdia-engine
   git remote remove origin
   git remote add origin git@github.com:aracdia/aracdia-engine.git
   git checkout -b main
   git push -u origin main
   ```

> Keep the upstream remote as `upstream` if you want to merge Luanti updates:
> ```bash
> git remote add upstream https://github.com/luanti-org/luanti.git
> ```

## Step 2 — Rebrand the basics (optional first pass)

Edit these files and rename mentions of "Minetest" / "Luanti" → "Aracdia"
where it makes sense:

- `README.md`, `LICENSE.txt` (keep Luanti notice — we're LGPL-bound)
- `src/defaultsettings.cpp` (default window title, port, etc.)
- `misc/aracdia.appdata.xml` and `.desktop` (Linux integration)
- App icons under `misc/`

Add an `ARACDIA-CHANGES.md` listing your modifications to satisfy LGPL.

You can also defer this — the launcher works with vanilla Luanti binaries
re-named under our convention.

## Step 3 — Build for all targets

The simplest reliable approach is GitHub Actions with a release workflow.
Skeleton (`.github/workflows/release.yml`):

```yaml
name: release

on:
  push:
    tags:
      - "v*"

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - { os: macos-latest,  target: aarch64-apple-darwin }
          - { os: macos-13,      target: x86_64-apple-darwin }
          - { os: windows-latest,target: x86_64-pc-windows-msvc }
          - { os: ubuntu-22.04,  target: x86_64-unknown-linux-gnu }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      # Install build deps (CMake, IrrlichtMt, etc.) — see Luanti's docs:
      # https://docs.luanti.org/for-engine-devs/building/
      - name: Build engine
        run: |
          # cmake -S . -B build -DCMAKE_BUILD_TYPE=Release ...
          # cmake --build build -j
          # collect binaries + textures + locale into ./out
          echo "Replace this block with the actual Luanti build commands."
      - name: Package
        shell: bash
        run: |
          cd out
          zip -r ../aracdia-engine-${{ matrix.target }}.zip .
          cd ..
          shasum -a 256 aracdia-engine-${{ matrix.target }}.zip \
            | awk '{print $1}' > aracdia-engine-${{ matrix.target }}.zip.sha256
      - uses: softprops/action-gh-release@v2
        if: startsWith(github.ref, 'refs/tags/')
        with:
          files: |
            aracdia-engine-${{ matrix.target }}.zip
            aracdia-engine-${{ matrix.target }}.zip.sha256
```

## Step 4 — Publish the first release

```bash
git tag v0.1.0   # or whatever Luanti version you're starting from + your suffix
git push origin v0.1.0
```

When the workflow finishes, the release page should contain 8 assets (4
zips + 4 sha256). The launcher's "INSTALLER ET JOUER" button will resolve
that release automatically.

## Step 5 — Test from the launcher

1. Run the launcher (`npm run tauri dev`).
2. Open Paramètres → Installation → confirm the manifest URL points to the
   right repo.
3. Back to Home → click **INSTALLER ET JOUER**.
4. The launcher downloads, verifies, extracts under
   `~/Library/Application Support/com.Aracdia.Launcher/engine/` (macOS) and
   reports "Moteur installé".

## Troubleshooting

- **"no engine asset published for target ..."** — the release is missing the
  zip for your OS+arch. Add it (or relax the matrix above).
- **"checksum mismatch"** — the `.sha256` sidecar doesn't match the zip.
  Re-run the workflow; the `shasum -a 256 | awk '{print $1}'` line is the
  only correct format.
- **"server returned 403"** — GitHub API rate-limit. Authenticate the launcher
  (planned for a later step) or wait an hour.
