-- Mapgen aliases.
--
-- All Luanti mapgens look up named aliases (e.g. `mapgen_stone`) at world-gen
-- time. A game without these crashes on the first chunk generation. The
-- mandatory set is small for v7/flat/valleys; we register a generous set
-- so the more complex mapgens (carpathian, fractal) also work without
-- spamming "mapgen alias missing" warnings.

local A = core.register_alias

A("mapgen_stone",                "aracdia_core:stone")
A("mapgen_water_source",         "aracdia_core:water_source")
A("mapgen_river_water_source",   "aracdia_core:water_source")

-- v6 (legacy) mapgen wants a few more — register them so users picking v6
-- don't get fatal errors. They reuse our existing nodes for now.
A("mapgen_dirt",                 "aracdia_core:dirt")
A("mapgen_dirt_with_grass",      "aracdia_core:grass")
A("mapgen_sand",                 "aracdia_core:sand")
A("mapgen_lava_source",          "aracdia_core:stone") -- placeholder, no lava yet
A("mapgen_cobble",               "aracdia_core:stone")
A("mapgen_mossycobble",          "aracdia_core:stone")
A("mapgen_tree",                 "aracdia_core:wood")
A("mapgen_leaves",               "aracdia_core:leaves")
A("mapgen_apple",                "aracdia_core:leaves")
A("mapgen_jungletree",           "aracdia_core:wood")
A("mapgen_jungleleaves",         "aracdia_core:leaves")
A("mapgen_pinetree",             "aracdia_core:wood")
A("mapgen_pineneedles",          "aracdia_core:leaves")
A("mapgen_dirt_with_snow",       "aracdia_core:dirt")
