-- Always-day environment: frozen noon, visible sun, no moon/night cycle.
--
-- Time progression is stopped via `time_speed = 0` in the launcher server
-- config; this module sets the initial time-of-day and per-player sky/sun.

local TIME_OF_DAY = 0.5 -- noon

local SKY = {
    type = "regular",
    clouds = true,
    sky_color = {
        day_sky = "#61B5F5",
        day_horizon = "#90D3F6",
        dawn_sky = "#61B5F5",
        dawn_horizon = "#90D3F6",
        night_sky = "#61B5F5",
        night_horizon = "#90D3F6",
        indoors = "#646464",
        fog_sun_tint = "#F47D1D",
        fog_moon_tint = "#7F99CC",
        fog_tint_type = "default",
    },
}

local function apply_player_sky(player)
    if not player or not player:is_player() then return end
    if not player.set_sky then return end

    player:set_sky(SKY)

    if player.set_sun then
        player:set_sun({
            visible = true,
            sunrise_visible = false,
            scale = 1,
        })
    end
    if player.set_moon then
        player:set_moon({ visible = false })
    end
    if player.set_stars then
        player:set_stars({ visible = false })
    end
end

local function lock_time()
    local tod = core.get_timeofday()
    if tod == nil then return end
    if math.abs(tod - TIME_OF_DAY) > 0.001 then
        core.set_timeofday(TIME_OF_DAY)
    end
end

core.register_on_joinplayer(function(player)
    apply_player_sky(player)
end)

-- Safety net if another mod or command moves time forward.
core.register_globalstep(function()
    lock_time()
end)

core.log("action", "[aracdia_core] always-day environment active")
