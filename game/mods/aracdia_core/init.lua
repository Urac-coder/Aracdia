-- Aracdia Core mod entry point.
--
-- This mod owns the foundational node registry, mapgen aliases and biome
-- definitions for the Aracdia game. It uses the modern `core` namespace
-- (alias of the legacy `minetest` namespace) for forward-compat.

local modpath = core.get_modpath(core.get_current_modname())

dofile(modpath .. "/nodes.lua")
dofile(modpath .. "/aliases.lua")
dofile(modpath .. "/biomes.lua")

core.log("action", "[aracdia_core] initialised")
