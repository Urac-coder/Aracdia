-- Build Mode — Aracdia's per-player creative flavour.
--
-- Toggling Build Mode grants four privileges:
--   * fly      — "K" by default
--   * fast     — "J" by default
--   * noclip   — "H" by default
--   * creative — recognised by some hooks; we still implement the powers
--                ourselves so we don't depend on engine globals
--
-- Powers wired up here (regardless of the world's `creative_mode` setting):
--   1. Invincibility — a `register_on_player_hpchange` modifier that swallows
--      every negative delta.
--   2. Instabreak — a `register_on_punchnode` hook that synchronously digs
--      the punched node, so players don't have to hold the dig button.
--   3. Infinite blocks — a `register_on_placenode` hook that refunds one
--      stack of the placed item back to the player's inventory.
--
-- The build_mode flag is persisted in `player:get_meta()` so it survives
-- reconnects.

local B = {}
aracdia_menu.build_mode = B

local META_KEY = "aracdia_build_mode"
local PRIVS = { fly = true, fast = true, noclip = true, creative = true }

--- Whether `name` is currently in Build Mode.
function B.is_on(name)
    local s = aracdia_menu.state.get(name)
    return s.build_mode == true
end

--- Sets Build Mode for `player` (granting/revoking privs and persisting).
function B.set(player, enabled)
    if not player or not player:is_player() then return end
    local name = player:get_player_name()
    local state = aracdia_menu.state.get(name)
    state.build_mode = enabled and true or false

    local privs = core.get_player_privs(name)
    for k in pairs(PRIVS) do
        privs[k] = enabled and true or nil
    end
    core.set_player_privs(name, privs)

    player:get_meta():set_int(META_KEY, enabled and 1 or 0)

    core.chat_send_player(
        name,
        enabled
            and "[Aracdia] Mode construction activé — vol/vitesse/traverser/instabreak/invincibilité"
            or "[Aracdia] Mode construction désactivé"
    )
end

--- Restore the persisted build_mode on join (and re-apply the privs in case
--- they were lost in transit, e.g. a cleared world).
core.register_on_joinplayer(function(player)
    local meta = player:get_meta()
    if meta:get_int(META_KEY) == 1 then
        B.set(player, true)
    end
end)

--- Invincibility hook — runs as a modifier (last arg = true) so we can
--- override the actual delta. Positive HP changes (e.g. healing) pass
--- through untouched.
core.register_on_player_hpchange(function(player, hp_change, _reason)
    if hp_change >= 0 then return hp_change end
    if not player or not player:is_player() then return hp_change end
    if B.is_on(player:get_player_name()) then
        return 0
    end
    return hp_change
end, true)

--- Instabreak hook. We mirror what `node_dig` does (drops, sounds, hooks)
--- so build-mode digs are indistinguishable from a held-dig completion.
core.register_on_punchnode(function(pos, node, puncher)
    if not puncher or not puncher:is_player() then return end
    if not B.is_on(puncher:get_player_name()) then return end
    core.node_dig(pos, node, puncher)
end)

--- Infinite blocks. After a successful place, we add one of the placed node
--- back to the placer's inventory so the displayed stack count never goes
--- down. We deliberately don't refund tools or craftitems — only nodes.
core.register_on_placenode(function(_pos, newnode, placer)
    if not placer or not placer:is_player() then return end
    if not B.is_on(placer:get_player_name()) then return end
    if not newnode or not newnode.name then return end
    local def = core.registered_nodes[newnode.name]
    if not def then return end
    local inv = placer:get_inventory()
    if inv then
        inv:add_item("main", ItemStack(newnode.name))
    end
end)
