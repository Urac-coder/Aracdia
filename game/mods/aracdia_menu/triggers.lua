-- Menu triggers — how the player opens the Aracdia menu.
--
-- Three entry points (none of them is Esc, because Luanti hardcodes Esc to
-- the engine pause menu — that work belongs to the C++ fork):
--   1. `/menu` chat command (also `/m` alias)
--   2. Aux1 + Sneak combo (E + Shift on default keymap), debounced 800 ms
--   3. A small HUD hint shown at join time so the player knows they exist

local COMBO_DEBOUNCE_US = 800 * 1000

local function open_menu(player)
    aracdia_menu.menu.show(player)
end

-- Chat commands
core.register_chatcommand("menu", {
    description = "Ouvre le menu du jeu Aracdia",
    func = function(name)
        local player = core.get_player_by_name(name)
        if not player then return false, "Joueur introuvable." end
        open_menu(player)
        return true
    end,
})
core.register_chatcommand("m", {
    description = "Alias de /menu",
    func = function(name)
        return core.chatcommands["menu"].func(name)
    end,
})

-- Aux1 + Sneak combo. Detected on each globalstep with a per-player
-- debounce: a long key press should open the menu once, not on every tick.
core.register_globalstep(function(_dtime)
    local now = core.get_us_time()
    for _, player in ipairs(core.get_connected_players()) do
        local ctrl = player:get_player_control()
        if ctrl.aux1 and ctrl.sneak then
            local s = aracdia_menu.state.get(player:get_player_name())
            if (now - (s.last_combo_trigger_us or 0)) > COMBO_DEBOUNCE_US then
                s.last_combo_trigger_us = now
                open_menu(player)
            end
        end
    end
end)

-- HUD hint (purely informational). Anchored to the bottom-center, faint
-- grey. Not interactive — Luanti HUD elements are not clickable.
core.register_on_joinplayer(function(player)
    player:hud_add({
        hud_elem_type = "text",
        position = { x = 0.5, y = 1 },
        offset = { x = 0, y = -16 },
        text = "/menu  ·  Aux1 + Sneak  →  ouvrir le menu Aracdia",
        scale = { x = 100, y = 30 },
        alignment = { x = 0, y = -1 },
        number = 0xC0C0C0,
    })
end)
