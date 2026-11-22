# Changelog

## 0.2.0 — 22 November 2026

### Gameplay
- The game catches up on up to ten missed ticks after a stall, as the 1.8
  timer does. Before, ticks lost during a busy moment (a burst of chunks, a
  crowded fight) were dropped, so movement ran under 20 ticks per second and
  felt heavy.
- The game thread's inbox has no size limit any more. When it was full, the
  network thread stopped, Keep Alive was answered late, and the game stopped
  ticking while it waited.
- The local player moves with the 1.8 float maths and angle tables, which
  stops small differences from turning into server setbacks.
- Collisions with slabs, stairs, boats, pistons and the world border, a 0.6
  block step, slime blocks, and digging with a sword in hand, as in 1.8.
- An Entity Teleport of your own player moves you and sends nothing back, as
  in 1.8; some servers use it to check the client.
- Riding horses, minecarts and boats.

### Mods
- New: Speed, Item Counter (splash potions, golden apples, ender pearls,
  arrows), Clock, Memory, 1.7 Animations, Direction HUD and Capes.
- Keystrokes, Armor Status and Potion Effects show item and effect pictures.
- The mods menu shows each mod as a tile with a large icon and an
  enabled/disabled bar, four to a row on a wide screen.

### Resource packs and visuals
- Connected textures, custom items, custom skies, custom colours, random and
  custom entity models, custom GUIs and fonts, and texture animations from
  OptiFine and MCPatcher packs.
- Dynamic Lights, Smart Animations, a lagometer, and video settings split
  into Quality, Details, Performance, Animations and Other pages.
- Three themes: Ember (the default), Amethyst and Vanilla.

### Other
- Signs and the sign editor, jukeboxes, fire on the player, spawner
  particles, Discord Rich Presence and the F3 key combinations.
- Screenshots and other off-screen renders use Direct3D 12 on Windows; the
  Vulkan driver of Intel integrated GPUs crashed on them.

## 0.1.0 — 15 November 2026

First release.

- Launch time on a Pentium G5400 with UHD 610 graphics went from 4.9 s to
  2.2 s: textures and packs load while the window and the GPU start, the
  Minecraft jar is read once, and PNGs decode on every core.
- Direct3D 12 by default on Windows.
- Reach Display, Combo Counter and the Toggle Sprint indicator.
- The scoreboard stays after a respawn into another dimension, and long chat
  messages no longer disconnect you.
