# Changelog

## 0.4.0 — 23 November 2026

### Performance
- The sky is drawn after the blocks, so its layers (dome, sunrise, stars,
  a pack's custom sky) only cover the pixels no block hides. On an Intel
  UHD 610 at 1600x900 a frame needs 25 % less GPU time, and the benchmark
  scene runs at 166 FPS instead of 123.
- New **Render Scale** option (Video Settings → Performance): the world is
  drawn at 25 to 100 % of the window resolution and stretched, while the
  hand, the HUD and all text stay sharp. The Low preset uses 50 %.
  `--render-scale N` sets it for one run.
- The benchmark reports the CPU time of the scene update (entity meshing).

### Inventory and containers
- Clicking outside an inventory window drops the item, and the pick-block
  key works in inventories, as in 1.8.
- Anvil cost and result, villager trades (used-up offers crossed out),
  the enchanting table's book, clues and costs, beacon tooltips, and the
  horse in its window.
- Item tooltips gain books, fireworks, attributes, blocks, and the
  advanced (F3 + H) lines; the written book gets its glint.

### Rendering
- The grass side overlay shows again over the dirt side.

## 0.3.0 — 22 November 2026

### Install
- A Windows installer, `cindral-setup-0.3.0.exe` (16 MB). It installs per
  user without an administrator prompt, adds Start menu and desktop
  shortcuts, and downloads the Minecraft 1.8.8 game files and sounds from
  Mojang as its last step. Updating is running the new installer.
- Without those files, the game now downloads them by itself on the first
  launch and shows the progress on the title screen. Textures and sounds
  switch on as soon as the download ends, with no restart.
- The game files are found even when Cindral is started by a launcher or a
  terminal that does not pass `APPDATA`. Before, such a start showed pink
  and black blocks, no items, no armor and no sounds.
- `cindral.exe` has its own icon, and it no longer opens a console window
  next to the game. Run from a terminal, it still prints there.

### Start
- A loading screen with the Cindral logo and a moving bar replaces the black
  window of the first second. The window opens only once it can draw it.
- The first start tunes the video settings to the machine: the graphics
  chip, the CPU threads and the memory pick the PvP, Medium or High preset,
  and the choice is saved like any other setting. A Pentium with Intel UHD
  graphics starts on the PvP preset.
- `--preset` now applies its settings; it was read and then ignored.

### Discord
- The Rich Presence card shows the Cindral logo, the version and its author,
  the server you play on when it has a public name (never a raw IP address),
  the time played, and a "Get Cindral" button.

### HUD
- Widgets have rounder corners, more room around the text and a thin
  outline that keeps them readable against a bright sky. The outline has its
  own colour and opacity, and can be turned off per widget.
- The title screen says "Cindral 0.3.0 by AeroMV".

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
