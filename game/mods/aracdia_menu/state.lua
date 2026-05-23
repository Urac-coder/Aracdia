-- Per-player runtime state for the Aracdia menu.
--
-- Volatile (in-memory) state lives here; durable values (build_mode flag) are
-- additionally mirrored into `player:get_meta()` so they survive restarts.

local M = {}
aracdia_menu.state = M

local players = {}

local function fresh()
    return {
        build_mode = false,
        last_combo_trigger_us = 0,
        creative_query = "",
        creative_page = 1,
    }
end

--- Returns the per-player state, lazily creating it on first access.
function M.get(name)
    local s = players[name]
    if not s then
        s = fresh()
        players[name] = s
    end
    return s
end

--- Wipes a player's state on leave so we don't leak memory across sessions
--- with thousands of historical players.
function M.clear(name)
    players[name] = nil
end

core.register_on_leaveplayer(function(player)
    M.clear(player:get_player_name())
end)
