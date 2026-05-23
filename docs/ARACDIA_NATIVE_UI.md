# Aracdia native in-game UI (engine fork)

Status: **approved direction** — replace long-term reliance on Luanti formspecs
with a client-side UI renderer embedded in `aracdia-engine`.

Related: [ARCHITECTURE.md](./ARCHITECTURE.md), [CREATE_ARACDIA_ENGINE.md](./CREATE_ARACDIA_ENGINE.md).

---

## Why

Formspecs are server-driven, layout-fragile, and visually limited. Aracdia needs
inventory, combat HUD, and menus that feel like a commercial RPG — closer to
Albion Online than Minecraft. That requires rendering UI in the **client process**
(C++), while gameplay rules stay **server-authoritative** (Lua).

Java / WebView inside the game client are poor fits:

| Approach | Verdict |
|---|---|
| Java | No JVM in Luanti; unrelated stack |
| WebView in client | CEF/WebView2 weight, GPU/input conflicts, huge maintenance |
| **RmlUi in engine** | HTML/CSS-like, game-UI provenance, GPU-friendly |
| ImGui in engine | Excellent for dev tools; less ideal for styled player UI |
| Formspecs (current) | Keep as fallback during migration |

**Recommendation: RmlUi** for player-facing screens; optional **ImGui** layer for
engine/mod debugging only.

---

## Repositories

| Repo | Role |
|---|---|
| [`Urac-coder/aracdia-engine`](https://github.com/Urac-coder/aracdia-engine) | C++ — renderer, input, network packets, Lua API surface |
| `Aracdia` (this monorepo) | Lua mods consume the API; RML/CSS assets; launcher unchanged |

The launcher already downloads tagged `aracdia-engine` releases. Each UI milestone
= new engine tag; game mod guards features with capability detection.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     aracdia-engine (client)                  │
│  ┌─────────────┐   ┌──────────────┐   ┌──────────────────┐  │
│  │ IrrlichtMt  │   │ RmlUi context│   │ Input router     │  │
│  │ 3D world    │──▶│ .rml + .rcss │◀──│ Esc / I / mouse  │  │
│  └─────────────┘   └──────┬───────┘   └──────────────────┘  │
│                           │ bind data models                   │
│                    ┌──────▼───────┐                            │
│                    │ UI manager   │◀── TOCLIENT_UI_* packets │
│                    └──────────────┘                            │
└────────────────────────────┬────────────────────────────────────┘
                             │ Luanti protocol (bump per feature)
┌────────────────────────────▼────────────────────────────────────┐
│                     Luanti server (Aracdia game)                   │
│  aracdia_menu / aracdia_ui (Lua)                                 │
│  • authoritative inventory, equipment, actions                   │
│  • core.open_aracdia_ui(player, "inventory", data)               │
│  • core.register_on_aracdia_ui_action(fn)                        │
└──────────────────────────────────────────────────────────────────┘
```

### Authority model

- **Server** owns item lists, validation, sort/stack, build mode gates.
- **Client** owns pixels, animations, hover, drag preview.
- Drag-and-drop: client sends intended move → server validates → pushes fresh
  snapshot (`TOCLIENT_UI_INVENTORY`) or rejects.

Same pattern as today, but the view layer moves from formspec strings to RmlUi
documents.

---

## Network (sketch)

Follow the existing pause-menu pattern (`TOSERVER_PAUSE_MENU`, protocol bump).

| Packet | Direction | Purpose |
|---|---|---|
| `TOCLIENT_ARACDIA_UI_OPEN` | S→C | `{ screen, json_state }` — load RML document |
| `TOCLIENT_ARACDIA_UI_UPDATE` | S→C | partial state patch |
| `TOCLIENT_ARACDIA_UI_CLOSE` | S→C | dismiss screen |
| `TOSERVER_ARACDIA_UI_ACTION` | C→S | `{ screen, action, payload }` |

Protocol version bump when shipped; old clients fall back to formspec path.

---

## Lua API (sketch)

Server-side only (mirrors `register_on_pause_menu` style):

```lua
-- Capability probe (game mod)
if core.register_on_aracdia_ui_action then
  -- native UI available
end

-- Open inventory UI for player
core.open_aracdia_ui(player, "inventory", {
  name = player:get_player_name(),
  load_pct = 42,
  estimate = 1200,
  main = serialize_stacks(inv:get_list("main")),
  equip = serialize_equip(player),
  build_mode = true,
})

core.register_on_aracdia_ui_action(function(player, screen, action, payload)
  if screen == "inventory" and action == "sort" then
    aracdia_menu.inventory.sort_main(player)
    core.open_aracdia_ui(player, "inventory", build_inventory_state(player))
    return true
  end
end)
```

Client-side Lua stays minimal or empty — rendering is C++.

---

## Engine integration points (Luanti fork)

Likely touch points in `aracdia-engine` (paths approximate, upstream Luanti layout):

1. **CMake** — add RmlUi (+ dependencies: FreeType, optionally HarfBuzz).
2. **`src/client/clientlauncher.cpp` / game loop** — init/shutdown RmlUi after Irrlicht device.
3. **`src/client/gameui/` (new)** — `AracdiaUIManager`: load documents, route input, draw overlay pass after HUD.
4. **`src/client/input/`** — when UI open, consume mouse/keyboard before player movement (like formspec menu).
5. **`src/network/`** — new opcodes + serializers (JSON via existing `jsoncpp` in tree).
6. **`src/script/lua_api/`** — `l_client` stays client; new server APIs in `l_server` or `l_aracdia`.
7. **Fallback** — if client lacks native UI, server keeps formspec path (current `inventory.lua`).

Reference prior patch: Esc → `TOSERVER_PAUSE_MENU` + `register_on_pause_menu` (v0.2.0).

---

## Asset pipeline (monorepo)

```
game/mods/aracdia_ui/
  rml/
    inventory.rml
    pause_menu.rml
  rcss/
    aracdia_theme.rcss      # shared tokens matching launcher (indigo, dark glass)
  textures/
    ui/                     # optional bitmap fonts / icons
  init.lua                  # bridge: native UI if available, else formspec fallback
```

RmlUi documents can reuse the same colour tokens as `aracdia_menu/textures/` and
the launcher Tailwind theme (`#6366F1`, `#0A0A0F`, etc.).

---

## Phased roadmap

### C.0 — Decision & spike (1–2 weeks)

- [x] Clone `aracdia-engine`, build locally (macOS arm64 first).
- [x] Integrate **RmlUi** with Irrlicht GL3 renderer backend.
- [x] Render a static `"Aracdia UI"` document over the world.
- [x] Toggle overlay with **F8**; no network yet.

**Exit criterion:** overlay visible in-game — test with local engine build (see below).

#### Local dev (C.0)

```bash
cd /Users/lucas/Documents/GitHub/aracdia-engine
mkdir -p build && cd build
cmake .. -DRUN_IN_PLACE=TRUE -DENABLE_ARACDIA_RMLUI=ON -DBUILD_SERVER=FALSE
make -j$(sysctl -n hw.logicalcpu)
# binary: ../bin/luanti
```

Copy or symlink into launcher engine dir, or run directly against local server.

In-game: **F8** toggles the RmlUi spike panel.

### C.1 — Input + lifecycle (1 week)

- [ ] UI manager open/close stack.
- [ ] Mouse capture, Esc to close top screen.
- [ ] Disable camera movement while UI focused.

### C.2 — Protocol + Lua open (2 weeks)

- [ ] Define opcodes; bump protocol version.
- [ ] `core.open_aracdia_ui` / `core.close_aracdia_ui` server-side.
- [ ] Client receives JSON, binds to RmlUi data model.
- [ ] Release `aracdia-engine` v0.3.0; launcher auto-updates.

### C.3 — Pause menu (1 week)

- [ ] Port pause menu to `pause_menu.rml` (parity with current formspec).
- [ ] Wire Esc → native UI when handler registered.
- [ ] Formspec fallback if client old.

### C.4 — Inventory v1 (2–3 weeks)

- [ ] Grid + equipment slots in RmlUi.
- [ ] Server-driven drag validation.
- [ ] Sort / stack / construction picker entry.
- [ ] Deprecate `set_inventory_formspec` when native path active.

### C.5 — Polish & cross-platform (ongoing)

- [ ] Windows + Linux CI builds.
- [ ] Font loading (Inter or bundled TTF).
- [ ] Animations, sound hooks.
- [ ] ImGui debug overlay (inventory state, packet log).

---

## Migration strategy (game mod)

`aracdia_menu/inventory.lua` and `menu.lua` become **fallback implementations**:

```lua
local function open_inventory(player)
  if core.open_aracdia_ui then
    core.open_aracdia_ui(player, "inventory", I.build_state(player))
  else
    I.apply_formspec(player)
  end
end
```

No big-bang switch — players on old engine keep formspecs.

---

## Risks

| Risk | Mitigation |
|---|---|
| RmlUi + Irrlicht glue complexity | Time-boxed C.0 spike; abort to retained-mode custom UI if blocked |
| LGPL compliance | `ARACDIA-CHANGES.md`, publish fork sources, link in launcher |
| Protocol drift | Capability probe + formspec fallback |
| Duplicate UI logic | Single `build_inventory_state()` Lua module feeds native or formspec |
| macOS code signing / notarization | Extend existing launcher quarantine strip; sign engine binaries in CI |

---

## Immediate next steps

1. Confirm **RmlUi** as player UI library (vs ImGui-first).
2. Confirm **first screen**: inventory (Albion-like) vs pause menu (smaller).
3. Set up local `aracdia-engine` build on macOS.
4. Execute **C.0 spike** in the engine repo; tag `v0.3.0-alpha.1` for internal test.

---

## Suggested commit (monorepo)

```
docs(engine): add native UI architecture plan for aracdia-engine fork
```
