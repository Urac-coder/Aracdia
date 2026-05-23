-- Player inventory — dark glass shell, same charter as menu.lua (Aracdia UI).

local I = {}
aracdia_menu.inventory = I

local C = {
    backdrop   = "#0A0A0FCC",
    shell      = "aracdia_ui_shell.png",
    slot       = "aracdia_ui_slot.png",

    text       = "#F5F5FA",
    text_dim   = "#A8A8B8",
    title      = "#E0E7FF",
    rule       = "#2A2A38",

    btn_pri    = "#6366F1",
    btn_pri_h  = "#818CF8",
    btn_pri_p  = "#4F46E5",

    btn_sec    = "#1A1A26",
    btn_sec_h  = "#22222E",
}

local TRANSPARENT = "#00000000"

local W = 7.6
local PX = 0.52
local PY = 0.52
local INNER = W - 2 * PX
local SLICE = 24
local RULE_H = 0.02
local BTN_H = 0.58
local ROW_GAP = 0.16
local SEC_GAP = 0.26

local SLOT_SIZE = 0.72
local SLOT_GAP = 0.08
local SLOT_PITCH = SLOT_SIZE + SLOT_GAP

local GRID_COLS = 4
local GRID_ROWS = 6
local MAIN_SIZE = GRID_COLS * GRID_ROWS
local MAX_LOAD = 100

local EQUIP_LAYOUT = {
    { list = "equip_head",    col = 0, row = 0 },
    { list = "equip_cape",    col = 1, row = 0 },
    { list = "equip_bag",     col = 2, row = 0 },
    { list = "equip_weapon",  col = 0, row = 1 },
    { list = "equip_chest",   col = 1, row = 1 },
    { list = "equip_offhand", col = 2, row = 1 },
    { list = "equip_potion",  col = 0, row = 2 },
    { list = "equip_boots",   col = 1, row = 2 },
    { list = "equip_food",    col = 2, row = 2 },
    { list = "equip_mount",   col = 1, row = 3 },
}

local function block_extent(cols, rows)
    return cols * SLOT_SIZE + (cols - 1) * SLOT_GAP,
        rows * SLOT_SIZE + (rows - 1) * SLOT_GAP
end

local function hrule(fs, y)
    fs[#fs + 1] = ("box[%f,%f;%f,%f;%s]"):format(PX, y, INNER, RULE_H, C.rule)
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

local function text_at(fs, id, x, y, w, h, text, color, size, bold, halign)
    local weight = bold and "bold" or "normal"
    local align = halign or "left"
    fs[#fs + 1] = ("style[%s;bgcolor=%s;border=false;noselect=true;textcolor=%s;halign=%s;font=%s;font_size=%s]")
        :format(id, TRANSPARENT, color, align, weight, size)
    fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;border=false;textcolor=%s;halign=%s]")
        :format(id, TRANSPARENT, color, align)
    fs[#fs + 1] = ("style[%s:pressed;bgcolor=%s;border=false;textcolor=%s;halign=%s]")
        :format(id, TRANSPARENT, color, align)
    fs[#fs + 1] = ("button[%f,%f;%f,%f;%s;%s]"):format(x, y, w, h, id, text)
end

local function solid_btn(fs, id, bg, bg_h, bg_p, fg)
    fs[#fs + 1] = ("style[%s;bgcolor=%s;textcolor=%s;border=false;halign=center;font_size=+12]")
        :format(id, bg, fg)
    fs[#fs + 1] = ("style[%s:hovered;bgcolor=%s;textcolor=%s;border=false;halign=center]")
        :format(id, bg_h, fg)
    fs[#fs + 1] = ("style[%s:pressed;bgcolor=%s;textcolor=%s;border=false;halign=center]")
        :format(id, bg_p, fg)
end

local function stacks_mergeable(a, b)
    if a:is_empty() or b:is_empty() then return false end
    if a:get_name() ~= b:get_name() or a:get_wear() ~= b:get_wear() then
        return false
    end
    return a:get_meta():equals(b:get_meta())
end

local function count_main_items(inv)
    local total = 0
    for _, stack in ipairs(inv:get_list("main") or {}) do
        if not stack:is_empty() then
            total = total + stack:get_count()
        end
    end
    return total
end

local function estimate_value(inv)
    return count_main_items(inv) * 10
end

function I.ensure_inventories(player)
    local inv = player:get_inventory()
    if not inv then return end

    inv:set_size("main", MAIN_SIZE)
    for _, slot in ipairs(EQUIP_LAYOUT) do
        inv:set_size(slot.list, 1)
    end
end

function I.stack_main(player)
    local inv = player:get_inventory()
    if not inv then return end

    local main = inv:get_list("main") or {}
    local changed = false

    for i = 1, #main do
        local stack = main[i]
        if not stack:is_empty() then
            local max_stack = stack:get_stack_max()
            for j = i + 1, #main do
                if stack:get_count() >= max_stack then break end
                local other = main[j]
                if stacks_mergeable(stack, other) then
                    local space = max_stack - stack:get_count()
                    local moved = math.min(space, other:get_count())
                    stack:set_count(stack:get_count() + moved)
                    other:set_count(other:get_count() - moved)
                    if other:get_count() == 0 then
                        main[j] = ItemStack("")
                    else
                        main[j] = other
                    end
                    changed = true
                end
            end
            main[i] = stack
        end
    end

    if not changed then return end

    local compact = {}
    for _, stack in ipairs(main) do
        if not stack:is_empty() then
            compact[#compact + 1] = stack
        end
    end
    while #compact < #main do
        compact[#compact + 1] = ItemStack("")
    end
    inv:set_list("main", compact)
end

function I.sort_main(player)
    local inv = player:get_inventory()
    if not inv then return end

    local main = inv:get_list("main") or {}
    local stacks = {}

    for _, stack in ipairs(main) do
        if not stack:is_empty() then
            stacks[#stacks + 1] = stack
        end
    end

    table.sort(stacks, function(a, b)
        local na, nb = a:get_name(), b:get_name()
        if na ~= nb then return na < nb end
        return a:get_count() > b:get_count()
    end)

    local sorted = {}
    for _, stack in ipairs(stacks) do
        sorted[#sorted + 1] = stack
    end
    while #sorted < #main do
        sorted[#sorted + 1] = ItemStack("")
    end
    inv:set_list("main", sorted)
end

local function render(player)
    local name = player:get_player_name()
    local build = aracdia_menu.build_mode.is_on(name)
    local inv = player:get_inventory()
    local load = inv and count_main_items(inv) or 0
    local load_pct = math.min(100, math.floor((load / MAX_LOAD) * 100))
    local estimate = inv and estimate_value(inv) or 0

    local equip_w = block_extent(3, 1)
    local equip_x0 = (W - equip_w) / 2
    local grid_w = block_extent(GRID_COLS, 1)
    local grid_x = (W - grid_w) / 2
    local _, equip_h = block_extent(3, 4)
    local _, grid_h = block_extent(GRID_COLS, GRID_ROWS)

    local y = PY + 0.06
    local title_y = y
    y = y + 0.48
    local sub_y = y
    y = y + 0.36 + SEC_GAP

    local rule1_y = y
    y = y + RULE_H + 0.20

    local equip_lbl_y = y
    y = y + 0.30 + 0.14
    local equip_y0 = y
    y = equip_y0 + equip_h + SEC_GAP

    local rule2_y = y
    y = y + RULE_H + 0.18

    local charge_lbl_y = y
    y = y + 0.28
    local bar_y = y
    y = bar_y + 0.14 + SEC_GAP

    local rule3_y = y
    y = y + RULE_H + 0.18

    local bag_lbl_y = y
    y = y + 0.28 + 0.14
    local grid_y = y
    y = grid_y + grid_h + SEC_GAP

    local rule4_y = y
    y = y + RULE_H + 0.18

    local build_y = y
    if build then
        y = y + BTN_H + 0.16
    end

    local estimate_y = y + 0.04
    local btn_y = estimate_y + 0.40
    y = btn_y + BTN_H + PY
    local H = y

    local fs = {
        "formspec_version[7]",
        ("size[%f,%f]"):format(W, H),
        "position[0.5,0.5]",
        "anchor[0.5,0.5]",
        "padding[0.06,0.06]",
        ("bgcolor[%s;true]"):format(C.backdrop),
        "no_prepend[]",
        ("background9[0,0;%f,%f;%s;false;%d]"):format(W, H, C.shell, SLICE),
        ("style_type[list;size=%f;spacing=%f]"):format(SLOT_SIZE, SLOT_GAP),
        ("style_type[list;slot_bg_img=%s;slot_bg_middle=4]"):format(C.slot),
        ("image[%f,%f;0.86,0.86;aracdia_ui_logo.png]"):format(PX, PY + 0.08),
    }

    text_at(fs, "inv_name", PX + 1.02, title_y, INNER - 1.02, 0.48, name, C.title, "+18", true, "left")
    text_at(fs, "inv_sub", PX + 1.02, sub_y, INNER - 1.02, 0.36, "Inventaire", C.text_dim, "+12", false, "left")

    hrule(fs, rule1_y)
    text_row(fs, "inv_equip_lbl", equip_lbl_y, 0.30, "Équipement", C.text_dim, "+11", false)

    for _, slot in ipairs(EQUIP_LAYOUT) do
        local x = equip_x0 + slot.col * SLOT_PITCH
        local ey = equip_y0 + slot.row * SLOT_PITCH
        fs[#fs + 1] = ("list[current_player;%s;%f,%f;1,1;]"):format(slot.list, x, ey)
    end

    hrule(fs, rule2_y)
    text_at(fs, "inv_charge_lbl", PX, charge_lbl_y, 1.5, 0.28, "Charge", C.text_dim, "+11", false, "left")
    text_at(fs, "inv_weight_pct", W - PX - 1.0, charge_lbl_y, 1.0, 0.28,
        load_pct .. "%", C.text, "+11", true, "right")

    fs[#fs + 1] = ("box[%f,%f;%f,0.14;%s]"):format(PX, bar_y, INNER, C.rule)
    local bar_w = INNER * (load_pct / 100)
    if bar_w > 0.04 then
        fs[#fs + 1] = ("box[%f,%f;%f,0.14;%s]"):format(PX, bar_y, bar_w, C.btn_pri)
    end

    hrule(fs, rule3_y)
    text_row(fs, "inv_bag_lbl", bag_lbl_y, 0.28, "Sac", C.text_dim, "+11", false)

    fs[#fs + 1] = ("list[current_player;main;%f,%f;%d,%d;]"):format(grid_x, grid_y, GRID_COLS, GRID_ROWS)
    fs[#fs + 1] = "listring[current_player;main]"
    for _, slot in ipairs(EQUIP_LAYOUT) do
        fs[#fs + 1] = ("listring[current_player;%s]"):format(slot.list)
    end

    hrule(fs, rule4_y)

    if build then
        solid_btn(fs, "aracdia_blocks", C.btn_pri, C.btn_pri_h, C.btn_pri_p, C.text)
        fs[#fs + 1] = ("button[%f,%f;%f,%f;aracdia_blocks;Blocs construction]")
            :format(PX, build_y, INNER, BTN_H)
    end

    text_at(fs, "inv_estimate", PX, estimate_y, INNER, 0.34,
        ("Estimation VM  %s"):format(tostring(estimate)), C.text_dim, "+10", false, "left")

    local btn_w = 1.42
    local btn_x = W - PX - (btn_w * 2 + ROW_GAP)
    solid_btn(fs, "inv_stack", C.btn_sec, C.btn_sec_h, C.btn_sec, C.text)
    solid_btn(fs, "inv_sort", C.btn_sec, C.btn_sec_h, C.btn_sec, C.text)
    fs[#fs + 1] = ("button[%f,%f;%f,%f;inv_stack;Empiler]"):format(btn_x, btn_y, btn_w, BTN_H)
    fs[#fs + 1] = ("button[%f,%f;%f,%f;inv_sort;Trier]"):format(btn_x + btn_w + ROW_GAP, btn_y, btn_w, BTN_H)

    return table.concat(fs)
end

function I.apply(player)
    if not player or not player:is_player() then return end
    player:set_inventory_formspec(render(player))
end

core.register_on_joinplayer(function(player)
    I.ensure_inventories(player)
    I.apply(player)
end)

core.register_on_player_receive_fields(function(player, formname, fields)
    if formname ~= "" then return false end

    if fields.inv_name or fields.inv_sub or fields.inv_equip_lbl or fields.inv_bag_lbl
        or fields.inv_charge_lbl or fields.inv_weight_pct or fields.inv_estimate then
        return true
    end

    if fields.inv_stack then
        I.stack_main(player)
        I.apply(player)
        return true
    end

    if fields.inv_sort then
        I.sort_main(player)
        I.apply(player)
        return true
    end

    if not fields.aracdia_blocks then return false end

    local pname = player:get_player_name()
    if not aracdia_menu.build_mode.is_on(pname) then
        core.chat_send_player(pname,
            "[Aracdia] Les blocs construction ne sont disponibles qu'en Mode construction.")
        return true
    end

    aracdia_menu.creative.show(player)
    return true
end)
