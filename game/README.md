# Aracdia — game content

This folder contains the Luanti (Minetest) "game" that is shipped to players by
the launcher. Mods inside `mods/` implement Aracdia's signature mechanics:

- `core_inventory/` — weight-based inventory (replaces stack-based logic)
- `core_combat/` — MOBA-like spell system (4–5 spells per class, cooldowns, skillshots)
- `core_ui/` — custom HUD and menus (non-Minecraft visual style)
- `core_world/` — world generation, biomes, mobs

Will be packaged as a versioned zip and downloaded by the launcher at runtime.
