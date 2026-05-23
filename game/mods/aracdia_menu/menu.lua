-- Aracdia main menu — formspec rendering and event dispatch.
--
-- Visual identity aligned with the Tauri launcher: dark surfaces, indigo
-- accent, 9-slice textures for readable buttons (plain bgcolor alone is
-- too subtle on the engine's default formspec renderer).

local M = {}
aracdia_menu.menu = M

local FORM_MAIN = "aracdia_menu:main"
M.FORM_MAIN = FORM_MAIN

-- 9-slice margin shared by all UI textures (see gen_placeholders.py).
local SLICE = 4

-- Launcher design tokens.
local C = {
    backdrop    = "#0A0A0FCC",
    text        = "#F5F5FA",
    text_dim    = "#A8A8B8",
    text_muted  = "#6A6A78",
    accent      = "#818CF8",
    success     = "#34D399",
    danger      = "#F87171",
}

local T = {
    panel      = "aracdia_ui_panel.png",
    header     = "aracdia_ui_header.png",
    divider    = "aracdia_ui_divider.png",
    primary    = "aracdia_ui_btn_primary.png",
    primary_h  = "aracdia_ui_btn_primary_h.png",
    primary_p  = "aracdia_ui_btn_primary_p.png",
    secondary  = "aracdia_ui_btn_secondary.png",
    secondary_h = "aracdia_ui_btn_secondary_h.png",
    success    = "aracdia_ui_btn_success.png",
    success_h  = "aracdia_ui_btn_success_h.png",
    danger     = "aracdia_ui_btn_danger.png",
    danger_h   = "aracdia_ui_btn_danger_h.png",
    disabled   = "aracdia_ui_btn_disabled.png",
}

--- Apply 9-slice button styling for `id`.
local function style_button(fs, id, normal, hovered, pressed, textcolor)
    fs[#fs + 1] = ("style[%s;bgimg=%s;bgimg_hovered=%s;bgimg_pressed=%s;bgimg_middle=%d;border=false;bgcolor=#00000000;textcolor=%s]")
        :format(id, normal, hovered, pressed, SLICE, textcolor)
end

--- Builds the formspec string for the main menu.
local function render(name)
    local build = aracdia_menu.build_mode.is_on(name)

    local W, H = 8.2, 6.6
    local pad_x = 0.35
    local inner_w = W - 2 * pad_x
    local btn_h = 0.82
    local cx = W / 2

    local fs = {
        "formspec_version[7]",
        ("size[%f,%f]"):format(W, H),
        ("bgcolor[%s;true]"):format(C.backdrop),
        "no_prepend[]",

        -- Card surface (9-slice panel).
        ("background9[0,0;%f,%f;%s;false;%d]"):format(W, H, T.panel, SLICE),

        -- Accent strip under the title block.
        ("background9[%f,0.42;%f,0.22;%s;false;2]"):format(pad_x + 0.05, inner_w - 0.1, T.header),

        -- Header — centred, no unicode clutter.
        ("style[hdr_title;textcolor=%s;font=bold;font_size=+22;halign=center]"):format(C.accent),
        ("label[%f,0.72;ARACDIA]"):format(cx),
        ("style[hdr_sub;textcolor=%s;font=normal;font_size=+12;halign=center]"):format(C.text_dim),
        ("label[%f,1.18;Menu du jeu]"):format(cx),

        -- Separator below header.
        ("background9[%f,1.62;%f,0.06;%s;false;1]"):format(pad_x, inner_w, T.divider),

        "style_type[button;font_size=+14]",
        "style_type[label;font=normal;font_size=+14]",
    }

    local function button(y, id, label, kind)
        if kind == "primary" then
            style_button(fs, id, T.primary, T.primary_h, T.primary_p, C.text)
        elseif kind == "success" then
            style_button(fs, id, T.success, T.success_h, T.success, C.success)
        elseif kind == "danger" then
            style_button(fs, id, T.danger, T.danger_h, T.danger, C.danger)
        elseif kind == "disabled" then
            style_button(fs, id, T.disabled, T.disabled, T.disabled, C.text_muted)
        else
            style_button(fs, id, T.secondary, T.secondary_h, T.secondary, C.text)
        end
        fs[#fs + 1] = ("button[%f,%f;%f,%f;%s;%s]"):format(pad_x, y, inner_w, btn_h, id, label)
    end

    local y = 1.88
    local gap = 0.14

    button(y, "resume", "Reprendre", "primary")
    y = y + btn_h + gap

    button(y, "build_toggle", "Mode construction", build and "success" or "secondary")
    y = y + btn_h + gap

    if build then
        button(y, "creative_inv", "Inventaire creatif", "secondary")
    else
        button(y, "creative_disabled", "Inventaire creatif", "disabled")
    end

    -- Bottom separator + destructive action pinned to the card footer.
    local sep_y = H - 1.35
    fs[#fs + 1] = ("background9[%f,%f;%f,0.06;%s;false;1]"):format(pad_x, sep_y, inner_w, T.divider)
    button(sep_y + 0.18, "quit_game", "Quitter le jeu", "danger")

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
        return true
    end
    if fields.resume then
        M.hide(name)
        return true
    end
    if fields.build_toggle then
        aracdia_menu.build_mode.set(player, not aracdia_menu.build_mode.is_on(name))
        M.show(player)
        return true
    end
    if fields.creative_inv then
        aracdia_menu.creative.show(player)
        return true
    end
    if fields.creative_disabled then
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
