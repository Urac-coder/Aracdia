-- Aracdia main menu — formspec rendering and event dispatch.
--
-- Visual identity: "mystic violet". Dark surface tinted purple, vivid
-- violet primary actions (#A855F7), subtle border accents. Designed to
-- match the magic/spell theme of Aracdia. Built on formspec_version 7
-- so we can use modern style[btn:hovered] / style[btn:pressed] selectors
-- and keep visual hierarchy with `box` panels.
--
-- The previous "Menu Luanti" button has been removed: with the v0.2.0
-- engine patch, Esc routes here, so closing this menu and pressing Esc
-- would just re-open it. If we want to expose Luanti's vanilla menu later
-- it'll need a dedicated `core.show_native_pause_menu` API patched into
-- the engine fork.

local M = {}
aracdia_menu.menu = M

local FORM_MAIN = "aracdia_menu:main"
M.FORM_MAIN = FORM_MAIN

-- Aracdia palette. Centralised so any UI tweak is a one-line change.
local C = {
    backdrop    = "#0A061280", -- dim world behind the form (RGBA)
    surface     = "#15101E",   -- main card
    surface_alt = "#1F1830",   -- inset / hovered button
    border      = "#2D1F44",   -- 1px-style separators and frame
    accent      = "#A855F7",   -- primary action (violet)
    accent_hot  = "#C084FC",   -- hovered primary
    accent_dim  = "#7E22CE",   -- pressed primary
    success     = "#34D399",   -- "Build mode ON" indicator
    danger      = "#F87171",   -- "Quit"
    danger_bg   = "#3A1818",   -- hovered Quit background
    text        = "#F5F3FF",   -- primary text
    text_dim    = "#A5A0C0",   -- subtitle text
}

--- Builds the formspec string for the main menu.
local function render(name)
    local build = aracdia_menu.build_mode.is_on(name)

    -- Card layout. Slightly taller than before to breathe.
    local W, H = 10, 10.5
    local pad = 0.4
    local inner_x = 0.6
    local inner_w = W - 1.2
    local btn_h = 0.95

    local fs = {
        "formspec_version[7]",
        ("size[%f,%f]"):format(W, H),
        ("bgcolor[%s;true]"):format(C.backdrop),

        -- Surface card.
        ("box[%f,%f;%f,%f;%s]"):format(pad, pad, W - 2 * pad, H - 2 * pad, C.surface),

        -- Frame: 4 thin boxes simulate a 1px border. formspec has no
        -- "stroke" so we draw it manually. Width 0.025 reads as a single
        -- crisp line on default DPI.
        ("box[%f,%f;%f,0.025;%s]"):format(pad, pad, W - 2 * pad, C.border),
        ("box[%f,%f;%f,0.025;%s]"):format(pad, H - pad - 0.025, W - 2 * pad, C.border),
        ("box[%f,%f;0.025,%f;%s]"):format(pad, pad, H - 2 * pad, C.border),
        ("box[%f,%f;0.025,%f;%s]"):format(W - pad - 0.025, pad, H - 2 * pad, C.border),

        -- Header: ARACDIA in big violet, "Menu du jeu" subtitle.
        ("style_type[label;textcolor=%s;font=bold;font_size=*2.2]"):format(C.accent),
        ("label[%f,1.45;ARACDIA]"):format(inner_x),
        ("style_type[label;textcolor=%s;font=normal;font_size=*0.95]"):format(C.text_dim),
        ("label[%f,2.05;Menu du jeu]"):format(inner_x),

        -- Top separator under the header.
        ("box[%f,2.55;%f,0.025;%s]"):format(inner_x, inner_w, C.border),

        -- Reset label style for any later label.
        "style_type[label;font=normal;font_size=*1]",
    }

    -- Adds a styled button with hover + pressed selectors.
    local function button(y, id, label, kind)
        if kind == "primary" then
            fs[#fs + 1] = ("style[%s;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.accent, C.text)
            fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.accent_hot, C.text)
            fs[#fs + 1] = ("style[%s:pressed;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.accent_dim, C.text)
        elseif kind == "active" then
            fs[#fs + 1] = ("style[%s;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.surface_alt, C.success)
            fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.border, C.success)
        elseif kind == "danger" then
            fs[#fs + 1] = ("style[%s;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.surface_alt, C.danger)
            fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.danger_bg, C.danger)
        else -- ghost
            fs[#fs + 1] = ("style[%s;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.surface_alt, C.text)
            fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;textcolor=%s;border=false]"):format(id, C.border, C.text)
        end
        fs[#fs + 1] = ("button[%f,%f;%f,%f;%s;%s]"):format(inner_x, y, inner_w, btn_h, id, label)
    end

    -- Vertical action stack. Layout is fixed regardless of build_mode so
    -- the "Quit" button never moves under the player's mouse — predictable
    -- click targets matter.
    local y = 2.95
    local gap = 0.20

    button(y, "resume", "▶   Reprendre", "primary")
    y = y + btn_h + gap

    button(y, "build_toggle",
        build and "◆   Mode construction  ·  ACTIVÉ"
              or  "◇   Mode construction  ·  désactivé",
        build and "active" or "ghost")
    y = y + btn_h + gap

    -- Creative inventory only makes sense when Build mode is on. Reserve
    -- the slot but render an inactive label when off, to keep layout
    -- stable.
    if build then
        button(y, "creative_inv", "▣   Inventaire créatif", "ghost")
    else
        fs[#fs + 1] = ("style[creative_disabled;bgcolor=%s;textcolor=%s;border=false]"):format(
            C.surface, "#4D4763")
        fs[#fs + 1] = ("button[%f,%f;%f,%f;creative_disabled;▣   Inventaire créatif (Build mode requis)]")
            :format(inner_x, y, inner_w, btn_h)
    end

    -- Bottom separator above the destructive action.
    local sep_y = H - 1.65
    fs[#fs + 1] = ("box[%f,%f;%f,0.025;%s]"):format(inner_x, sep_y, inner_w, C.border)

    -- Quit always pinned to the bottom of the card.
    button(H - 1.4, "quit_game", "×   Quitter le jeu", "danger")

    return table.concat(fs)
end

--- Show the main menu to a player.
function M.show(player)
    if not player or not player:is_player() then return end
    local name = player:get_player_name()
    core.show_formspec(name, FORM_MAIN, render(name))
end

--- Hide our menu (used by the "Reprendre" button).
function M.hide(name)
    core.close_formspec(name, FORM_MAIN)
end

core.register_on_player_receive_fields(function(player, formname, fields)
    if formname ~= FORM_MAIN then return false end
    local name = player:get_player_name()

    if fields.quit then
        -- Closed via Esc inside the form: the engine sets `quit=true`.
        return true
    end
    if fields.resume then
        M.hide(name)
        return true
    end
    if fields.build_toggle then
        aracdia_menu.build_mode.set(player, not aracdia_menu.build_mode.is_on(name))
        -- Re-render so the toggle's label/colour reflect the new state.
        M.show(player)
        return true
    end
    if fields.creative_inv then
        aracdia_menu.creative.show(player)
        return true
    end
    if fields.creative_disabled then
        -- Visual-only button; nudge the player toward Build mode.
        core.chat_send_player(name,
            "[Aracdia] Active d'abord le Mode construction pour acceder a l'inventaire creatif.")
        return true
    end
    if fields.quit_game then
        core.kick_player(name, "Tu as quitte le jeu via le menu Aracdia.")
        return true
    end
    return true
end)
