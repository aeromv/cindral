# Changelog

## 0.9.3 — 25 November 2026

### Anticheats
- Clicks in an inventory, chat lines, signs, books and the respawn button
  now leave inside the next tick, before its movement packet, as in 1.8.9
  (`GuiScreen.handleInput` runs inside `Minecraft.runTick`). They left the
  moment they were made, between two ticks, and GrimAC flagged the clicks
  as `Post` ("click window"): a click after the movement packet of a tick
  is what an inventory cheat sends.
- The 0.9.2 knockback fix also covers a batch that stops at its size limit
  (256 packets, on a crowded server or after a stutter): the knockback,
  explosions and teleports that already arrived behind it still come
  before the tick.

### Hits, plainly
- The README and the website said Cindral registers hits better than
  1.8.9. The server decides the hits; Cindral sends each click at the time
  1.8.9 would, using the aim of that moment. The wording now says so.

## 0.9.2 — 25 November 2026

### Smooth knockback again
- Since 0.9.0 a batch of packets that ran past the start of a tick left
  the rest of the batch for after that tick, to keep the ticks on time. If
  the player's knockback was in that rest, it was applied a tick late:
  the knockback felt heavy and uneven, and the server, which had expected
  it a tick earlier, pulled the player back to an older position. As in
  1.8.9, the knockback, explosions, teleports and the player's own input
  that have already arrived now always come before the tick, with every
  packet before them; only what follows them (chunks, mostly) waits.

### Logs
- When `logs/latest.log` cannot be replaced (another copy of the game holds
  it, for example), the session writes `logs/session-<number>.log`
  instead of going without a log.

## 0.9.1 — 25 November 2026

### The world no longer stays empty after joining
- Since 0.6.1 the chunk meshing workers ran below normal priority, so that
  they would not delay the ticks. Below normal, they also waited behind
  every other busy program on the PC: with a video wallpaper or a browser
  using the CPU, the world stayed empty for about ten seconds after a join
  on a two-core CPU. They are back at normal priority; the game and network
  threads stay above normal, which keeps the ticks on time on its own. With
  every CPU thread busy, the chunks around the player now show after
  1.4 s instead of 10.2 s, and the whole view after 4.6 s instead of 110 s.

### Camera
- In the first person the camera sits 0.1 block behind the eye, as in
  1.8.9 (`EntityRenderer.orientCamera`). It sat on the eye, so players and
  blocks close by looked slightly bigger than in 1.8.9.
- Under water the view narrows to 60/70 of the FOV, the hand included, as
  in 1.8.9.

### Entities
- Armour stands are drawn, with their pose, arms, base plate, worn armour,
  held item and head. Lobbies use them for displays and NPCs.
- The arrows stuck in your own body show in the third person and in the
  inventory portrait.
- The end portal surface and the beacon beam are drawn again.
- Picking up an item plays its pop, and the ender dragon beats its wings.

## 0.9.0 — 24 November 2026

### Fewer setbacks, steadier ping
- Ticks no longer start late behind a long batch of packets. Chunk columns
  are decoded on the game thread, and on a two-core CPU a batch of a few
  chunk packets ran past the start of the next tick; that tick then went
  out late, together with the next one. Servers read two movement packets
  at once as a timer cheat, or the knockback in them as taken a tick late,
  and pulled the player back (a session on a Pentium G5400 had 91 setbacks
  in 22 minutes, most of them on the steps of a knockback). The replies to
  the server's checks also waited, which shows as a higher ping. A batch
  now stops when a tick is due; the rest of the packets follow right after
  it, in the same order.
- `logs/latest.log` now explains setbacks: each one says how many ticks
  ago the last knockback and the last late tick were, and whether the
  player was sprinting or using an item. Once a minute the game thread
  writes how many ticks were late, how late, and how long its longest tick
  and packet batch took.

### The world appears faster
- Chunks go to the renderer nearest to the player first. When the server
  sends a whole view distance at once (joining, a respawn, a new map), they
  went in coordinate order, so the ground under the player could wait
  behind chunks far away.
- Chunks go to the renderer as soon as they arrive, not at the next tick.

### Resource packs
- Mini items packs work with the 1.7 Animations mod. The 1.7 position of a
  held item ignored the pack's `firstperson` pose; the pack's change from
  the vanilla pose (size and place) now applies on top of it.
- A pack zipped inside its own folder (`MyPack.zip/MyPack/assets/...`)
  loads. It loaded as an empty pack before.

### Updates
- The title screen says when a new version is out. One click downloads it
  in the background, checks it against the SHA-256 GitHub lists for it,
  installs it and restarts Cindral. A portable `cindral.exe` replaces
  itself.

## 0.8.0 — 24 November 2026

### Loading screen
- The loading screen always shows now. The game files often finished
  loading before the window's first frame; the game then went straight to
  the title screen while the window was still hidden, so the player saw
  nothing for a few seconds and then the menu.
- New design: the logo, a bar that follows the real loading steps (resource
  packs, block models, textures, font, upload), the name of the step and a
  percentage. It fades into the title screen.

### Faster end of loading
- The sky, custom sky and title pictures are decoded on the loading thread;
  the window only uploads them. A pack's custom sky is several large PNGs,
  and decoding them on the window thread froze it for half a second on a
  Pentium G5400.
- The brand font was rasterized a second time when the game files arrived;
  once is enough.
- The upload is done in two frames (block textures, then the rest), so the
  bar moves during it instead of stopping.

### Finding hitches
- A frame that takes over 100 ms in a game now writes its own line in
  `logs/latest.log`: how long it took and which part of the frame took the
  time (world, HUD, scene, render, or outside the drawing). The minute
  summary only said that hitches happened.

### README
- Rewritten in plain words: what Cindral is, how to install it, what is
  better than Java in PvP and what stays the same on purpose.

## 0.7.0 — 24 November 2026

### PvP: every click counts
- Two clicks inside one 50 ms tick are two attacks, as in 1.8 (its mouse
  event loop runs `clickMouse` for each press). Cindral kept one flag per
  tick and merged them, so a fast clicker lost hits: at 12 to 20 CPS two
  clicks often share a tick. A miss still blocks the clicks after it in the
  same tick, as vanilla's `leftClickCounter` does.
- The crosshair ray already uses the look of the moment, not one blended
  with the previous tick (the 1.8 delay that the MouseDelayFix mod
  removes).
- Attacks still leave at the start of a tick, as Java sends them: sending
  them earlier, between ticks, is what anticheats flag as a hacked client.

## 0.6.2 — 24 November 2026

- Only the game writes `logs/latest.log`. Benchmarks, demos and the
  installer's asset check replaced it before, losing the session a player
  wanted to send.
- The last five game sessions are kept: `latest.log`, `previous.log`, then
  `previous-2.log` to `previous-4.log`.

## 0.6.1 — 24 November 2026

### Smoother play on small CPUs
- The game thread (ticks) and the network thread run at above-normal
  priority, the chunk meshing workers below normal. On a two-core,
  four-thread CPU (a Pentium G5400) the busy threads outnumber the CPU's
  slots while chunks load; Windows shared them evenly, so ticks and packets
  waited behind mesh jobs, arrived in bunches and the camera jumped: a high
  frame rate that felt like 30.
- On four threads or fewer, two meshing workers instead of three, so the
  render, game and network threads keep room.
- The minute summary in `logs/latest.log` adds how evenly the ticks reach
  the render thread (P50/P95/P99 interval, repeated and collapsed ticks).

## 0.6.0 — 24 November 2026

### Display
- **Display** moved to the main Options page, across the bottom row. It
  opens a list of every resolution the monitor offers (read from the
  monitor, not a fixed list); a click picks one.
- **Desktop resolution on Alt+Tab** (on by default, on the Display page):
  leaving a fullscreen game at another resolution gives the desktop its
  own back at once, and the game takes its own again when you return.
- Leaving fullscreen always restores the desktop resolution and puts the
  window back where it was and at its size; a game started in fullscreen
  comes out centred at 1280x720 instead of somewhere off to the side.
- Super Secret Settings and the Twitch stream button are gone: nothing was
  behind them.

### Mods
- New **No Enchants**: no enchantment glint on swords, armour and items, in
  the hand, on players, on the ground and in the inventory; the glint pass
  is skipped entirely.
- **Hit Color** starts on a much darker red (#800000 at 60 %). A colour
  chosen before keeps its value.
- **Speed** is removed: blocks per second does not help in a fight.

### Knockback
- A burst of more than 256 server packets (chunks, a crowded spawn) no
  longer waits a frame between passes, so the knockback behind it is
  applied on time.

## 0.5.0 — 23 November 2026

### Fewer setbacks
- Server packets are never held longer than one tick, and always reach the
  game before its next tick. Before, a slow frame (a stutter) held the
  knockback and teleports of the server for its whole length while the
  game kept sending movement from the old state; many servers answered by
  pulling the player back.

### Display
- New **Display** option (Video Settings → Other): the screen resolution in
  fullscreen, from Desktop down to 800x600. A lower one switches the monitor
  to it and stretches the picture: softer, but far fewer pixels to draw.
  The window ignores it.
- Switching to another window keeps the frame rate. It used to drop to 30
  FPS, so coming back started on slow, stuttering frames.

### Hit Color
- New Hit Color mod, on by default: players and mobs you hit turn a deeper
  red than vanilla's pale tint. Colour and strength can be changed; turning
  the mod off brings back the vanilla tint.

### Logs
- `%APPDATA%\cindral\logs\latest.log` records each run (the previous one is
  kept as `previous.log`): every setback with how far the server pulled you
  and your ping, and once a minute the average FPS, the slowest frame and
  the number of stutters. Send it along with a bug report.

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
