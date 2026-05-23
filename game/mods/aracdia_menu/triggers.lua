-- Menu triggers — how the player opens the Aracdia menu.
--
-- Four entry points, in order of UX preference:
--   1. Esc — captured at the engine level. The Aracdia engine fork patches
--      Luanti so pressing Esc in-game sends TOSERVER_PAUSE_MENU instead of
--      opening the native menu. We intercept it via core.register_on_pause_menu.
--      If the engine is the vanilla upstream (no patch), this hook is never
--      called and the native pause menu opens — graceful degradation.
--   2. `/menu` chat command (also `/m` alias)
--   3. Aux1 + Sneak combo (E + Shift on default keymap), debounced 800 ms
--   4. A small HUD hint shown at join time so the player knows about #2/#3

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
        text = "Échap  ·  /menu  ·  Aux1 + Sneak  →  ouvrir le menu Aracdia",
        scale = { x = 100, y = 30 },
        alignment = { x = 0, y = -1 },
        number = 0xC0C0C0,
    })
end)

-- Aracdia engine fork hook: Esc in-game routes here instead of opening the
-- native pause menu. We open our custom menu and return true to claim the
-- request — the server then knows it doesn't need to fall back to the
-- engine's built-in menu. If the player is somehow not a real player,
-- return false so the engine still gives them the native menu as safety.
if core.register_on_pause_menu then
    core.register_on_pause_menu(function(player)
        if not player or not player:is_player() then return false end
        open_menu(player)
        return true
    end)
end
