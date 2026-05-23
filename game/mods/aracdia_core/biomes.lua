-- Single placeholder biome so the v7 / valleys / carpathian mapgens have
-- something coherent to populate the surface with. Sand on the coast,
-- grass on land, stone underneath.
--
-- This is intentionally minimal — the gameplay-driven biome system will
-- come later in dedicated mods (aracdia_world / aracdia_climate).

core.register_biome({
	name = "aracdia_plains",
	node_top = "aracdia_core:grass",
	depth_top = 1,
	node_filler = "aracdia_core:dirt",
	depth_filler = 3,
	node_stone = "aracdia_core:stone",
	node_riverbed = "aracdia_core:sand",
	depth_riverbed = 2,
	y_max = 31000,
	y_min = 1,
	heat_point = 50,
	humidity_point = 50,
})

core.register_biome({
	name = "aracdia_beach",
	node_top = "aracdia_core:sand",
	depth_top = 1,
	node_filler = "aracdia_core:sand",
	depth_filler = 2,
	node_stone = "aracdia_core:stone",
	y_max = 4,
	y_min = -2,
	heat_point = 50,
	humidity_point = 50,
})

core.register_biome({
	name = "aracdia_underwater",
	node_top = "aracdia_core:sand",
	depth_top = 1,
	node_filler = "aracdia_core:sand",
	depth_filler = 3,
	node_stone = "aracdia_core:stone",
	y_max = -3,
	y_min = -31000,
	heat_point = 50,
	humidity_point = 50,
})
