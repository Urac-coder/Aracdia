-- Aracdia in-game menu mod.
--
-- Owns:
--   * The custom main menu formspec (no Luanti vanilla pause menu)
--   * Build Mode (construction): fly, fast, noclip, instabreak, invincibility,
--     infinite block placement, construction block picker in player inventory
--   * Combat mode: default survival behaviour
--   * Triggers: `/menu` chat command and the Aux1+Sneak key combo
--
-- Server-only mod. Pure Lua, no engine modifications.

local modpath = core.get_modpath(core.get_current_modname())

aracdia_menu = {}

dofile(modpath .. "/state.lua")
dofile(modpath .. "/build_mode.lua")
dofile(modpath .. "/menu.lua")
dofile(modpath .. "/creative.lua")
dofile(modpath .. "/inventory.lua")
dofile(modpath .. "/triggers.lua")

core.log("action", "[aracdia_menu] initialised")
