-- Foundational nodes for Aracdia.
--
-- Group conventions (matched against tools later, kept compatible with the
-- common Luanti idiom):
--   * crumbly = soft soil materials (dirt, sand) — broken by shovels
--   * cracky  = stone-like — broken by picks
--   * choppy  = wood-like  — broken by axes
--   * snappy  = leaves, plants — broken by hand/shears
--
-- The numeric suffix is the digging difficulty (3 = easy, 1 = hard).

-- Node names follow the Luanti convention `<modname>:<localname>`. The
-- mod is `aracdia_core` so all nodes here live in that namespace. Future
-- mods (aracdia_combat, aracdia_world, …) will own their own prefix.

core.register_node("aracdia_core:dirt", {
	description = "Dirt",
	tiles = { "aracdia_dirt.png" },
	groups = { crumbly = 3, soil = 1 },
	is_ground_content = true,
})

core.register_node("aracdia_core:grass", {
	description = "Grass Block",
	tiles = {
		"aracdia_grass_top.png",
		"aracdia_dirt.png",
		"aracdia_grass_side.png",
		"aracdia_grass_side.png",
		"aracdia_grass_side.png",
		"aracdia_grass_side.png",
	},
	groups = { crumbly = 3, soil = 1 },
	drop = "aracdia_core:dirt",
	is_ground_content = true,
})

core.register_node("aracdia_core:stone", {
	description = "Stone",
	tiles = { "aracdia_stone.png" },
	groups = { cracky = 3, stone = 1 },
	drop = "aracdia_core:stone",
	is_ground_content = true,
})

core.register_node("aracdia_core:sand", {
	description = "Sand",
	tiles = { "aracdia_sand.png" },
	groups = { crumbly = 3, falling_node = 1, sand = 1 },
	is_ground_content = true,
})

-- Liquid nodes need a `_source` and `_flowing` pair, even for a stub.
core.register_node("aracdia_core:water_source", {
	description = "Water",
	drawtype = "liquid",
	tiles = { "aracdia_water.png" },
	special_tiles = { { name = "aracdia_water.png", backface_culling = false } },
	use_texture_alpha = "blend",
	paramtype = "light",
	walkable = false,
	pointable = false,
	diggable = false,
	buildable_to = true,
	is_ground_content = true,
	drop = "",
	drowning = 1,
	liquidtype = "source",
	liquid_alternative_flowing = "aracdia_core:water_flowing",
	liquid_alternative_source = "aracdia_core:water_source",
	liquid_viscosity = 1,
	post_effect_color = { a = 103, r = 30, g = 60, b = 90 },
	groups = { water = 3, liquid = 3, cools_lava = 1 },
})

core.register_node("aracdia_core:water_flowing", {
	description = "Flowing Water",
	drawtype = "flowingliquid",
	tiles = { "aracdia_water.png" },
	special_tiles = {
		{ name = "aracdia_water.png", backface_culling = false },
		{ name = "aracdia_water.png", backface_culling = true },
	},
	use_texture_alpha = "blend",
	paramtype = "light",
	paramtype2 = "flowingliquid",
	walkable = false,
	pointable = false,
	diggable = false,
	buildable_to = true,
	is_ground_content = true,
	drop = "",
	drowning = 1,
	liquidtype = "flowing",
	liquid_alternative_flowing = "aracdia_core:water_flowing",
	liquid_alternative_source = "aracdia_core:water_source",
	liquid_viscosity = 1,
	post_effect_color = { a = 103, r = 30, g = 60, b = 90 },
	groups = { water = 3, liquid = 3, not_in_creative_inventory = 1 },
})

core.register_node("aracdia_core:wood", {
	description = "Wood Log",
	tiles = {
		"aracdia_wood_top.png",
		"aracdia_wood_top.png",
		"aracdia_wood.png",
	},
	paramtype2 = "facedir",
	on_place = core.rotate_node,
	groups = { choppy = 2, oddly_breakable_by_hand = 1, wood = 1 },
})

core.register_node("aracdia_core:leaves", {
	description = "Leaves",
	drawtype = "allfaces_optional",
	tiles = { "aracdia_leaves.png" },
	use_texture_alpha = "clip",
	paramtype = "light",
	groups = { snappy = 3, leafdecay = 3, leaves = 1, flammable = 2 },
	drop = {
		max_items = 1,
		items = {
			{ items = { "aracdia_core:leaves" }, rarity = 5 },
		},
	},
	sounds = nil,
})
