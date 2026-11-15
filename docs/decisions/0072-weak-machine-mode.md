# ADR-0072 — Weak machines: face merging, render scale, presets

Status: proposed (2026-11-14)

## Context

Perf 2 (`docs/tasks/perf2/README.md`, phase F) left the weak-machine work
undone: no presets, no render scale, and "fast graphics" does nothing for
speed. Measured on a machine with **no GPU** (Mesa llvmpipe, the CPU
rasterizes; 4 cores), release build, `--bench-scene`, official 1.8.8 assets:

- 1280x720, default options: about 54 ms per frame (18-19 fps).
- `perf` shows about 70 % of the CPU in the four llvmpipe raster threads
  (fragment shading) and the rest in the one thread that runs the vertex
  shader and the triangle setup. At 320x180 the frame stays at 35 ms: there
  the serial vertex work is the floor.
- Skipping draws one group at a time: the SOLID terrain is two thirds of the
  frame, sky, clouds and translucent terrain a few ms each.
- In the fragment shader, the four dynamically indexed lightmap reads cost
  about 20 % of the frame.

An integrated GPU has the same shape at a smaller scale (F2 measured the
opaque terrain at 82 % of the GPU time on an Intel iGPU).

## Decision

Three levers, each one optional, plus presets that combine them.

1. **Face merging** (`umicraft_mesh::merge_faces`, option `merge_faces`, on by
   default). After meshing, coplanar neighbouring opaque quads with the same
   normal, texture layer, light payload (block, sky, AO, shade bit), tint and
   texture mapping are joined into larger quads, greedily (rows, then
   columns), per section. The texture repeats over a merged quad: its
   coordinates run past 1 and `sample_block` in `terrain.wgsl` wraps them with
   `fract`, keeping the derivatives of the unwrapped coordinates, so the mip
   level is unchanged. Coordinates inside `0..=1`, which every unmerged quad
   has, are not touched. Not merged: quads that are not axis-aligned
   rectangles, whose UVs are not affine in the position or leave `0..=1`, and
   exact rectangles that appear twice on one side of a plane (a base face and
   its overlay, such as the grass side). Quad order and corner order are kept;
   the section spans are rebuilt, so cave culling and the solid/cutout split
   are unchanged. UVs of a merged quad are computed relative to its first
   corner in f64, so they stay exact far from the origin.
   Image: on the bench scene 25 of 921 600 pixels differ (sub-texel rounding
   at texel edges; one was a pre-existing crack that merging closed). Faces:
   -33 % with the Low preset on the deliberately bumpy bench terrain; flat
   ground merges far more.
2. **Render scale** (`Renderer::set_render_scale`, option `render_scale`,
   25-100 %, default 100). The world pass draws into a smaller colour and
   depth target, stretched over the screen with nearest filtering; the hand,
   the screen overlays and the HUD are drawn after it at full resolution. At
   100 % nothing is allocated and the path is the old one. The menu blur and
   the motion blur keep the full-resolution path.
3. **Lightmap per vertex with flat lighting** (`Renderer::set_flat_lighting`,
   driven by `smooth_lighting == 0`). With flat lighting every opaque quad has
   one light level at its four corners, so looking the lightmap up in the
   vertex shader gives the same colour as per fragment. Only the opaque
   passes use it; the blended pass, items, entities and the hand keep the
   per-fragment lookup.
4. **Presets** `VideoPreset::{Low, Medium, High}`; `--preset NAME` on the
   command line (also for `--bench-scene`). On a **first start** (no
   `options.toml`), the window picks Low for a software adapter and Medium for
   an integrated GPU (`AdapterSummary::kind`), then saves the options; a
   graphics card keeps the reference defaults.

| | Low | Medium | High |
|---|---|---|---|
| render distance | 6 | 8 | 8 |
| render scale | 50 % | 75 % | 100 % |
| smooth lighting | off | minimum | maximum |
| clouds | off | fast | fancy |
| particles | minimal | decreased | all |
| fancy graphics, entity shadows, block alternatives | off | on | on |
| mipmaps | 2 | 4 | 4 |

## Results (llvmpipe, 4 cores, 1280x720, official assets, 200 frames, two rounds)

| Run | mean frame | fps |
|---|---|---|
| default, no merging | 53.7-56.3 ms | 18-19 |
| default (merging on) | 53.1-55.8 ms | 18-19 |
| `--preset medium` | 42.1-46.8 ms | 21-24 |
| `--preset low`, no merging | 26.0-27.2 ms | 37-38 |
| `--preset low` | 22.2-23.7 ms | 42-45 |

The vertex lightmap was measured on the procedural set: Low 20.1 to about
18 ms. All runs share a busy 4-core VM: about ±3 ms of noise.

## Consequences

- Render scale, face merging and the presets are not vanilla options; they
  change no pixel at 100 % with merging, and the Low/Medium presets only set
  vanilla options plus these two.
- A merged quad meets its unmerged neighbours at T-junctions. No crack was
  seen on the bench scene, but a rasterizer may show a rare one-pixel gap;
  `merge_faces = false` in `options.toml` turns merging off.
- For a moment after smooth lighting is switched off, meshes built with
  smooth lighting are drawn with the light of their provoking vertex until
  the re-mesh finishes.
- Next levers, not done: vanilla fast leaves (opaque, inner faces culled:
  the forests the bench has none of), entity mesh caching (perf 2 C2), and a
  lower Low scale on very large screens.
