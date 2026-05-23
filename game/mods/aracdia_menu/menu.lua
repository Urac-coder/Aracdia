-- Aracdia main menu — formspec rendering and event dispatch.
--
-- Luanti labels ignore halign=center on this engine fork; action buttons do
-- not. Header title/subtitle are therefore rendered as transparent, non-
-- interactive buttons so they share the same centring path as "Reprendre".

local M = {}
aracdia_menu.menu = M

local FORM_MAIN = "aracdia_menu:main"
M.FORM_MAIN = FORM_MAIN

local C = {
    backdrop   = "#0A0A0FCC",
    shell      = "aracdia_ui_shell.png",

    text       = "#F5F5FA",
    text_dim   = "#A8A8B8",
    title      = "#E0E7FF",
    rule       = "#2A2A38",

    btn_pri    = "#6366F1",
    btn_pri_h  = "#818CF8",
    btn_pri_p  = "#4F46E5",

    btn_sec    = "#1A1A26",
    btn_sec_h  = "#22222E",

    btn_on     = "#14532D",
    btn_on_h   = "#166534",
    btn_on_fg  = "#6EE7B7",

    danger     = "#F87171",
    danger_h   = "#FCA5A5",
}

local TRANSPARENT = "#00000000"

local PX      = 0.52
local PY      = 0.56
local W       = 7.2
local INNER   = W - 2 * PX
local SLICE   = 24

local BTN_H   = 0.68
local ROW_GAP = 0.18
local RULE_H  = 0.02

local function solid_btn(fs, id, bg, bg_h, bg_p, fg)
    fs[#fs + 1] = ("style[%s;bgcolor=%s;textcolor=%s;border=false;halign=center;font_size=+14]")
        :format(id, bg, fg)
    fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;textcolor=%s;border=false;halign=center]")
        :format(id, bg_h, fg)
    fs[#fs + 1] = ("style[%s:pressed;bgcolor=%s;textcolor=%s;border=false;halign=center]")
        :format(id, bg_p, fg)
end

local function text_row(fs, id, y, h, text, color, size, bold)
    local weight = bold and "bold" or "normal"
    fs[#fs + 1] = ("style[%s;bgcolor=%s;border=false;noselect=true;textcolor=%s;halign=center;font=%s;font_size=%s]")
        :format(id, TRANSPARENT, color, weight, size)
    fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;border=false;textcolor=%s;halign=center]")
        :format(id, TRANSPARENT, color)
    fs[#fs + 1] = ("style[%s:pressed;bgcolor=%s;border=false;textcolor=%s;halign=center]")
        :format(id, TRANSPARENT, color)
    fs[#fs + 1] = ("button[%f,%f;%f,%f;%s;%s]"):format(PX, y, INNER, h, id, text)
end

local function hrule(fs, y)
    fs[#fs + 1] = ("box[%f,%f;%f,%f;%s]"):format(PX, y, INNER, RULE_H, C.rule)
end

--- Label for the build/combat toggle — shows the mode you switch *to*.
local function mode_toggle_label(build)
    return build and "Mode combat" or "Mode construction"
end

local function render(name)
    local build = aracdia_menu.build_mode.is_on(name)

    local title_y = PY + 0.10
    local title_h = 0.55
    local sub_y   = title_y + title_h + 0.14
    local sub_h   = 0.40

    local y = sub_y + sub_h + 0.28

    local rule1_y = y
    y = y + RULE_H + 0.26

    local btn1_y = y
    y = y + BTN_H + ROW_GAP

    local btn2_y = y
    y = y + BTN_H + 0.32

    local rule2_y = y
    y = y + RULE_H + 0.22

    local quit_y = y
    y = y + 0.56

    local H = y + PY

    local fs = {
        "formspec_version[7]",
        ("size[%f,%f]"):format(W, H),
        "position[0.5,0.5]",
        "anchor[0.5,0.5]",
        "padding[0.06,0.06]",
        ("bgcolor[%s;true]"):format(C.backdrop),
        "no_prepend[]",

        ("background9[0,0;%f,%f;%s;false;%d]"):format(W, H, C.shell, SLICE),
    }

    text_row(fs, "hdr_title", title_y, title_h, "ARACDIA", C.title, "+22", true)
    text_row(fs, "hdr_sub", sub_y, sub_h, "Menu du jeu", C.text_dim, "+12", false)

    hrule(fs, rule1_y)

    solid_btn(fs, "resume", C.btn_pri, C.btn_pri_h, C.btn_pri_p, C.text)
    fs[#fs + 1] = ("button[%f,%f;%f,%f;resume;Reprendre]"):format(PX, btn1_y, INNER, BTN_H)

    if build then
        solid_btn(fs, "build_toggle", C.btn_on, C.btn_on_h, C.btn_on, C.btn_on_fg)
    else
        solid_btn(fs, "build_toggle", C.btn_sec, C.btn_sec_h, C.btn_sec, C.text)
    end
    fs[#fs + 1] = ("button[%f,%f;%f,%f;build_toggle;%s]"):format(
        PX, btn2_y, INNER, BTN_H, mode_toggle_label(build))

    hrule(fs, rule2_y)

    fs[#fs + 1] = ("style[quit_game;bgcolor=%s;border=false;textcolor=%s;halign=center;font_size=+13]")
        :format(TRANSPARENT, C.danger)
    fs[#fs + 1] = ("style[quit_game:hovered;bgcolor=#EF444418;border=false;textcolor=%s;halign=center]")
        :format(C.danger_h)
    fs[#fs + 1] = ("button[%f,%f;%f,0.50;quit_game;Quitter le jeu]"):format(PX, quit_y, INNER)

    return table.concat(fs)
end

function M.show(player)
    if not player or not player:is_player() then return end
    local name = player:get_player_name()
    core.show_formspec(name, FORM_MAIN, render(name))
end

function M.hide(name)
    core.close_formspec(name, FORM_MAIN)
end

core.register_on_player_receive_fields(function(player, formname, fields)
    if formname ~= FORM_MAIN then return false end
    local name = player:get_player_name()

    if fields.quit then
        return true
    end
    if fields.hdr_title or fields.hdr_sub then
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
    if fields.quit_game then
        core.kick_player(name, "Tu as quitte le jeu via le menu Aracdia.")
        return true
    end
    return true
end)
