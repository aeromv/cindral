# ADR-0073 — PvP mode: preset, Fast Render, PvP HUD mods, plan for 300 fps

Status: proposed (2026-11-14)

## Context

The target player is on a Pentium Gold G5400 (2 cores, 4 threads, 3.7 GHz,
no AVX) with its UHD 610 (12 execution units, Gen9.5), playing PvP on 1.8.9
servers. The vanilla 1.8.9 client gives about 40 fps there; Lunar Client 300 to
400 fps. The goal is 300 fps or more, and the HUD mods PvP players expect.

Umicraft speaks protocol 47, which every 1.8.x server (1.8.9 included) uses:
it already joins 1.8.9 servers as a 1.8.9 client does. The 1.8.8 and 1.8.9
asset sets are the same for the purpose of the client.

## Decision

1. **PvP preset** (`--preset pvp`, `VideoPreset::Pvp`): render distance 4,
   full resolution (a sharp picture for aim), no frame cap and no vsync, flat
   lighting, no clouds, decreased particles, fast graphics, no entity shadows,
   one variant per block, mipmaps 2, face merging.
2. **Hires lookup per vertex** (always on, exact): the layer of the large
   texture array (`hires_map`) is read once per vertex and passed flat,
   instead of a dynamically indexed uniform read per fragment.
3. **Fast Render** (option, experimental, off in every preset): the terrain's
   lightmap colour and fog share are computed per vertex and interpolated,
   like a fixed-function pipeline. No gain could be measured on llvmpipe (the
   only adapter available here); it is kept so it can be measured on an Intel
   GPU, where dynamically indexed uniform reads per fragment are costly.
4. **PvP HUD mods** (ADR-0065 framework): Keystrokes (W A S D, both mouse
   buttons with their clicks per second, the space bar; a held key lights up),
   Armor Status (each worn piece with what is left, green to red), Potion
   Effects (name, level in roman numerals, time left, in the potion colour,
   blinking in the last ten seconds). Grid widgets are a new `HudCell` /
   `ModCell` primitive drawn by `umicraft_ui::mod_widget`; the HUD editor
   measures them with `ModWidgetView::size`. The game thread publishes the
   worn armour and the effects in `HudState::mod_facts`, again only when an
   item, its damage, an effect, or a timer's second changes. Icons: Phosphor
   `keyboard`, `shield`, `flask` (MIT, as the others).

## Update (same day): PvP-client look and facing culling

The first Armor Status and Potion Effects were text lists. They now look as
in the PvP clients (Lunar's built-in status mods):

- A widget line can carry a picture (`HudIcon`: a worn armour slot, the held
  item, a potion effect) and a second, grey row (`detail`). The client turns
  the picture into the item's real icon (the same icon atlas as the hotbar)
  or the effect's status icon of `inventory.png` (18 x 18 at `(0, 198)`,
  `Potion.setIconIndex`), and `umicraft_ui::mod_widget` lays out one row per
  line, as tall as its icon. A widget on the right half of the screen is
  mirrored: text, then the icon against the edge.
- Armor Status: icon and the durability left in white (Lunar's default), the
  held item under the armour; options: value / percent / hidden, text colour
  or green-to-red, item names, held item.
- Potion Effects: status icon, name and level, the time under it in grey,
  blinking through the last ten seconds; options: icons, names, names in the
  potion colour, name and duration colours.
- Both float on the game with no panel by default (`hud_settings_bare!`).

**Facing culling** (Sodium's technique): a sorted opaque pass is laid out
solid then cutout, each by facing bucket (+X, -X, +Y, -Y, +Z, -Z, unaligned),
each bucket section by section, so a bucket is contiguous over the column.
At draw time a bucket is skipped in a section when the eye is on the back
side of the section's box on that axis (`visible_facings`): those quads face
away and the pipeline would cull them after shading their vertices. The image
is the same (0 differing pixels on the bench scene, default and PvP). On the
bench it draws 20 % fewer indices (469 339 against 583 280 per frame). A first
version that kept the section-major order split the runs and cost 2 to 3 ms
more on llvmpipe; the column-wide order removed that. No frame-time gain is
measurable on llvmpipe (fill bound at 720p); a GPU with little vertex
throughput (Intel Gen9) is where it should show. `--bench-no-facing-cull`
compares.

Research notes (2026-11-14): Lunar's Armor Status and Potion Effects are
built-in mods with movable pieces, icons and warning colours; Sodium's main
techniques are a compact vertex format, multi-draw batching and early culling
of chunk faces that face away from the camera; Rust Minecraft clients
(wgpu-mc, hypercubed) publish no comparable benchmarks.

## Measurements (no GPU: llvmpipe, 4 cores; official 1.8.8 assets; 200 frames)

| Run | 1280x720 | 1920x1080 |
|---|---|---|
| default options | 56.9-57.6 ms (17-18 fps) | — |
| `--preset low` | 25.2-27.8 ms (36-40 fps) | — |
| `--preset pvp` | 15.7-17.3 ms (58-64 fps) | 24.4-24.7 ms (41 fps) |

Fast Render on/off, three interleaved rounds: no difference beyond noise
(Low 25.2-27.8 ms either way, PvP 15.7-17.3 ms either way).

## Measured on the G5400 + UHD 610 (2026-11-14, Windows 10, DX12)

`--bench-scene --preset pvp`, release build, idle machine, 2 frames in
flight: 1920x1080 2.45 ms (about 408 fps, GPU 2.2 ms, CPU of the render call
0.8-1.3 ms); 1280x720 1.4 ms (about 715 fps). The terrain is GPU-bound at
1080p and balanced at 720p. Rows in `docs/perf-log.md`. Done from the plan
below, each measured:

- Vulkan crashed in the Intel driver; DX12 is the Windows default.
- The window keeps 2 frames in flight without vsync (215 to 284 fps at 1080p
  in the same session).
- GPU 4, differently: the sky is drawn after the opaque terrain on the far
  plane, so only the uncovered pixels pay for it (-6 % GPU, identical pixels).
- GPU 3: Fast Render gives -8 % GPU here; still off by default.

The bench has no trees, entities or HUD; the CPU items below are to be read
in game with the F3 `CPU` line.

## Plan to 300 fps on a G5400 (not done; to measure on the machine first)

300 fps is a 3.3 ms frame for CPU and GPU together. Measure first:
`umicraft-client --bench-scene --width 1920 --height 1080 --preset pvp`
(terrain only), then a real server with the F3 `CPU` line (world, hud, scene,
render).

GPU (UHD 610: about a tenth of the Iris Xe of the F2 baseline, often on
single-channel memory):
1. Vanilla fast leaves (F1 of perf 2): opaque leaves, inner faces culled.
2. Re-test the packed 16-byte vertex (F2-3c) on Gen9: it was slower on a
   Raptor Lake iGPU, but Gen9 on single-channel DDR4 is bandwidth bound.
3. Measure Fast Render there; make it the PvP default if it helps.
4. A PvP sky: one clear colour instead of the dome passes.

CPU (2 cores, 4 threads):
1. Cache entity meshes while their pose is unchanged (perf 2 C2), then pose
   on the GPU (C4): a PvP lobby has many players.
2. Finish the HUD cache: keep the tessellated triangles (B2), share the HUD
   state by `Arc` instead of cloning it (B3).
3. Thread budget on 2 cores: one mesh worker at below-normal priority while
   playing, more while loading.
4. `mimalloc` (E2), measured on the HUD and entity paths.
5. Release builds with the `dist` profile (fat LTO), `x86-64-v2` (the G5400
   has no AVX).

## Consequences

- PvP and Fast Render are not vanilla; the PvP preset only sets vanilla
  options plus render scale and merging.
- Keystrokes shows W A S D whatever the bindings are (as the PvP clients do).
- Effect names in Potion Effects are English; the menu texts are translated.
