-- Main menu formspec & button dispatch.
--
-- The Aracdia main menu replaces the bare engine pause UI for everything
-- the player can do *in-game*. Esc still opens the Luanti vanilla menu —
-- we surface a "Menu Luanti" button that closes ours so the next Esc press
-- routes there naturally.

local M = {}
aracdia_menu.menu = M

local FORM_MAIN = "aracdia_menu:main"
M.FORM_MAIN = FORM_MAIN

--- Builds the formspec string for the main menu.
local function render(name)
    local build = aracdia_menu.build_mode.is_on(name)
    local W, H = 8, 9.5
    local btn_w = 5
    local btn_h = 0.9
    local btn_x = (W - btn_w) / 2

    -- Centered card on a dimmed backdrop. `bgcolor` greys out the world
    -- behind the form; `box` paints our own panel on top.
    local fs = {
        "formspec_version[6]",
        ("size[%f,%f]"):format(W, H),
        "bgcolor[#0008;true]",
        ("box[0.4,0.4;%f,%f;#161B22]"):format(W - 0.8, H - 0.8),
        "style_type[label;textcolor=#E6EDF3]",
        ("label[%f,1.4;Aracdia]"):format(btn_x),
        "style_type[label;textcolor=#8B949E;font_size=12]",
        ("label[%f,2.0;Menu du jeu]"):format(btn_x),
        "style_type[label;textcolor=#E6EDF3;font_size=14]",
    }

    local function button(y, id, label, primary)
        if primary then
            fs[#fs + 1] = ("style[%s;bgcolor=#1F6FEB;textcolor=#FFFFFF]"):format(id)
        else
            fs[#fs + 1] = ("style[%s;bgcolor=#21262D;textcolor=#E6EDF3]"):format(id)
        end
        fs[#fs + 1] = ("button[%f,%f;%f,%f;%s;%s]"):format(btn_x, y, btn_w, btn_h, id, label)
    end

    button(2.9, "resume", "Reprendre", true)
    button(4.1, "build_toggle",
        build and "Mode construction : ACTIVÉ" or "Mode construction : désactivé",
        false)
    if build then
        button(5.3, "creative_inv", "Inventaire créatif", false)
    end
    button(6.7, "luanti_menu", "Menu Luanti (touche Échap)", false)
    button(7.9, "quit_game", "Quitter le jeu", false)

    return table.concat(fs)
end

--- Show the main menu to a player.
function M.show(player)
    if not player or not player:is_player() then return end
    local name = player:get_player_name()
    core.show_formspec(name, FORM_MAIN, render(name))
end

--- Hide our menu (used by the "Menu Luanti" button).
function M.hide(name)
    core.close_formspec(name, FORM_MAIN)
end

core.register_on_player_receive_fields(function(player, formname, fields)
    if formname ~= FORM_MAIN then return false end
    local name = player:get_player_name()

    if fields.quit then
        -- Player closed the form via Esc — engine fires this with `quit=true`.
        return true
    end
    if fields.resume then
        M.hide(name)
        return true
    end
    if fields.build_toggle then
        aracdia_menu.build_mode.set(player, not aracdia_menu.build_mode.is_on(name))
        -- Re-render the menu so the toggle label updates immediately.
        M.show(player)
        return true
    end
    if fields.creative_inv then
        aracdia_menu.creative.show(player)
        return true
    end
    if fields.luanti_menu then
        M.hide(name)
        core.chat_send_player(
            name,
            "[Aracdia] Appuie sur Échap pour ouvrir le menu Luanti."
        )
        return true
    end
    if fields.quit_game then
        core.kick_player(name, "Tu as quitté le jeu via le menu.")
        return true
    end
    return true
end)
