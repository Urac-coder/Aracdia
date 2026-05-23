-- Creative inventory (Build Mode only).
--
-- A paginated formspec that lists every registered item with a known
-- description, hiding the ones flagged `not_in_creative_inventory = 1`.
-- Searching narrows by substring in the technical name OR in the
-- description.
--
-- Click an item → 99 of it lands in the player's main inventory. Combined
-- with the `register_on_placenode` refund hook, this gives an effectively
-- infinite supply.

local C = {}
aracdia_menu.creative = C

local FORM_INV = "aracdia_menu:creative_inv"
C.FORM_INV = FORM_INV

local PER_PAGE_COLS = 8
local PER_PAGE_ROWS = 6
local PER_PAGE = PER_PAGE_COLS * PER_PAGE_ROWS

--- Returns a sorted list of item names eligible for the creative inventory.
local function eligible_items()
    local items = {}
    for name, def in pairs(core.registered_items) do
        if name ~= ""
            and name ~= "ignore"
            and name ~= "unknown"
            and name ~= "air"
            and (not def.groups or def.groups.not_in_creative_inventory ~= 1)
            and (def.description or "") ~= ""
        then
            items[#items + 1] = name
        end
    end
    table.sort(items)
    return items
end

--- Filters `items` (in place would mutate the cached list, so we copy).
local function filter(items, query)
    if not query or query == "" then return items end
    local q = query:lower()
    local filtered = {}
    for _, name in ipairs(items) do
        local def = core.registered_items[name]
        local desc = def and def.description or ""
        if name:lower():find(q, 1, true) or desc:lower():find(q, 1, true) then
            filtered[#filtered + 1] = name
        end
    end
    return filtered
end

local function render(name, query, page)
    local items = filter(eligible_items(), query)
    local total = #items
    local total_pages = math.max(1, math.ceil(total / PER_PAGE))
    page = math.max(1, math.min(page, total_pages))

    local W, H = 13, 10
    local fs = {
        "formspec_version[6]",
        ("size[%f,%f]"):format(W, H),
        "bgcolor[#0008;true]",
        ("box[0.4,0.4;%f,%f;#161B22]"):format(W - 0.8, H - 0.8),
        "style_type[label;textcolor=#E6EDF3]",
        "label[0.7,1.0;Inventaire créatif]",
        "style_type[label;textcolor=#8B949E;font_size=12]",
        ("label[0.7,1.5;Page %d / %d  ·  %d résultat%s]"):format(
            page, total_pages, total, total > 1 and "s" or ""
        ),
        "style_type[label;textcolor=#E6EDF3;font_size=14]",
        -- Search row
        ("field[0.7,2.1;6,0.8;query;Recherche;%s]"):format(core.formspec_escape(query or "")),
        "field_close_on_enter[query;false]",
        "style[search;bgcolor=#1F6FEB;textcolor=#FFFFFF]",
        "button[6.9,2.1;1.6,0.8;search;Filtrer]",
        "style[clear;bgcolor=#21262D;textcolor=#E6EDF3]",
        "button[8.7,2.1;1.6,0.8;clear;Effacer]",
        "style[back;bgcolor=#21262D;textcolor=#E6EDF3]",
        "button[10.5,2.1;1.6,0.8;back;Retour]",
    }

    local start_i = (page - 1) * PER_PAGE + 1
    local end_i = math.min(start_i + PER_PAGE - 1, total)
    local cell = 1.25
    local origin_x = 0.7
    local origin_y = 3.2

    for idx = start_i, end_i do
        local i = idx - start_i
        local x = origin_x + (i % PER_PAGE_COLS) * cell
        local y = origin_y + math.floor(i / PER_PAGE_COLS) * cell
        local item_name = items[idx]
        fs[#fs + 1] = ("item_image_button[%f,%f;%f,%f;%s;give_%d;]"):format(
            x, y, cell - 0.05, cell - 0.05, item_name, idx
        )
    end

    -- Pagination controls
    local nav_y = H - 1.1
    if page > 1 then
        fs[#fs + 1] = "style[prev;bgcolor=#21262D;textcolor=#E6EDF3]"
        fs[#fs + 1] = ("button[0.7,%f;1.8,0.8;prev;◀ Précédent]"):format(nav_y)
    end
    if page < total_pages then
        fs[#fs + 1] = "style[next;bgcolor=#21262D;textcolor=#E6EDF3]"
        fs[#fs + 1] = ("button[%f,%f;1.8,0.8;next;Suivant ▶]"):format(W - 2.5, nav_y)
    end

    -- Track per-player query/page so click/pagination know what to redraw.
    local s = aracdia_menu.state.get(name)
    s.creative_query = query or ""
    s.creative_page = page
    s.creative_items = items

    return table.concat(fs)
end

--- Show the creative inventory; refuses if the player isn't in build mode.
function C.show(player, query, page)
    if not player or not player:is_player() then return end
    local name = player:get_player_name()
    if not aracdia_menu.build_mode.is_on(name) then
        core.chat_send_player(
            name,
            "[Aracdia] L'inventaire créatif n'est accessible qu'en Mode construction."
        )
        return
    end
    local s = aracdia_menu.state.get(name)
    query = query or s.creative_query or ""
    page = page or s.creative_page or 1
    core.show_formspec(name, FORM_INV, render(name, query, page))
end

core.register_on_player_receive_fields(function(player, formname, fields)
    if formname ~= FORM_INV then return false end
    local name = player:get_player_name()
    local s = aracdia_menu.state.get(name)

    if fields.quit then
        return true
    end
    if fields.back then
        aracdia_menu.menu.show(player)
        return true
    end
    if fields.search then
        C.show(player, fields.query or "", 1)
        return true
    end
    if fields.clear then
        C.show(player, "", 1)
        return true
    end
    if fields.prev then
        C.show(player, s.creative_query, (s.creative_page or 1) - 1)
        return true
    end
    if fields.next then
        C.show(player, s.creative_query, (s.creative_page or 1) + 1)
        return true
    end
    -- Item click (`give_<idx>` field name).
    for k, _ in pairs(fields) do
        local idx = k:match("^give_(%d+)$")
        if idx then
            local item_idx = tonumber(idx)
            local items = s.creative_items or {}
            local item_name = items[item_idx]
            if item_name then
                local inv = player:get_inventory()
                if inv then
                    inv:add_item("main", ItemStack(item_name .. " 99"))
                    core.chat_send_player(
                        name,
                        ("[Aracdia] +99 %s"):format(item_name)
                    )
                end
            end
            return true
        end
    end
    return true
end)
