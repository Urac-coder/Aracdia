# Aracdia — game content

The Luanti (ex-Minetest) "game" shipped to players. The launcher places this
folder under the engine's user games directory at startup, then spawns the
engine with `--gameid aracdia`.

## Layout

```
game/
├── game.conf                  # game metadata (id=aracdia, title, description)
├── menu/                      # icons / backgrounds shown by the menu
│   ├── icon.png
│   ├── header.png
│   └── background.png
├── mods/
│   └── aracdia_core/          # foundational nodes, biomes, mapgen aliases
│       ├── mod.conf
│       ├── init.lua
│       ├── nodes.lua          # dirt, grass, stone, sand, water, wood, leaves
│       ├── aliases.lua        # mapgen aliases (mandatory for v6/v7/…)
│       ├── biomes.lua         # plains / beach / underwater placeholder biomes
│       └── textures/          # 16×16 RGBA placeholders generated from gen_placeholders.py
└── tools/
    └── gen_placeholders.py    # regenerate menu + texture PNGs (no external deps)
```

## Naming convention

All node and item names are prefixed with the mod name. The foundational
mod is `aracdia_core` so its nodes are `aracdia_core:dirt`, `aracdia_core:grass`,
etc. Future mods will own their own prefix:

- `aracdia_combat:fireball`, `aracdia_combat:cooldown_bar` (planned)
- `aracdia_inventory:weight_meter` (planned)
- `aracdia_ui:hud_*` (planned)

## Regenerating placeholder art

The texture PNGs are checked in for convenience but are produced by a script
that depends only on the Python 3 standard library:

```bash
python3 game/tools/gen_placeholders.py
```

Edit colours / patterns in `tools/gen_placeholders.py` (look for the
`TEXTURES` dict at the bottom) and re-run; the PNGs are written in place.

## Local dev: running the game without the launcher

Install the engine (the launcher does this automatically), then symlink this
folder into the engine's user games directory:

```bash
# On macOS:
ENGINE="$HOME/Library/Application Support/com.Aracdia.Launcher/engine"
mkdir -p "$HOME/Library/Application Support/minetest/games"
ln -s "$PWD/game" "$HOME/Library/Application Support/minetest/games/aracdia"

# Headless smoke test (~6s):
"$ENGINE/luanti.app/Contents/MacOS/luanti" --server \
    --world /tmp/aracdia_world --gameid aracdia --info
```

A clean run prints `[aracdia_core] initialised` and `Server for gameid="aracdia"
listening on [::]:30000`, with no `ERROR[Main]` lines.

## Roadmap

This is the foundation. Upcoming mods (intentionally separate so each can be
iterated on independently):

- `aracdia_inventory/` — weight-based inventory (replaces stack-based logic)
- `aracdia_combat/` — MOBA-like spell system (cooldowns, skillshots)
- `aracdia_ui/` — custom HUD and menus (distinct visual identity)
- `aracdia_world/` — generation, biomes, mobs, structures
