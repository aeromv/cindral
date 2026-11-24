<div align="center">

<img src=".github/assets/logo.png" width="88" alt="Cindral">

# Cindral

Minecraft 1.8.9 client for PvP, made by AeroMV.

**[Download](https://cindral.aeromv.com)** ·
[Releases](https://github.com/aeromv/cindral/releases) ·
[Changelog](CHANGELOG.md)

[![Release](https://img.shields.io/github/v/release/aeromv/cindral?color=ff7329)](https://github.com/aeromv/cindral/releases)
![Minecraft](https://img.shields.io/badge/Minecraft-1.8.9-3c8527)

<img src=".github/assets/title.png" alt="Cindral title screen" width="820">

</div>

Cindral is not Minecraft with mods added on top. It is a separate game
client, written in Rust, that connects to 1.8.8 and 1.8.9 servers exactly
like the normal game does. Because of that:

- you don't need Java,
- it opens in a couple of seconds,
- it runs well on cheap PCs and laptops.

## Download

1. Get `cindral-setup-<version>.exe` from
   [cindral.aeromv.com](https://cindral.aeromv.com) or the
   [latest release](https://github.com/aeromv/cindral/releases/latest).
2. Run it. No admin rights needed.
3. The installer downloads the Minecraft 1.8.8 textures and sounds from
   Mojang (about 110 MB). Cindral doesn't ship any Mojang files.

When a new version is out, the title screen tells you. Click **Update**:
it downloads in the background, installs, and restarts Cindral.

You can also use `cindral.exe` alone, without installing. Keep
`dxcompiler.dll` in the same folder so it starts faster.

## Performance

Tested on a cheap PC: Pentium Gold G5400, Intel UHD 610 (no graphics card),
1600x900, render distance 4, a 32x PvP pack.

| | FPS |
|---|---|
| Benchmark world | 166 |
| 20 players on screen | 160 |

If your PC is slow, open **Options → Display** and pick a lower resolution.
In fullscreen the game stretches it to fill the screen and you get a lot
more FPS.

The first time you start it, Cindral picks video settings that fit your PC.
You can change them later in **Options → Video Settings**.

## PvP

Things that work better than in Java 1.8.9:

- **No lost clicks.** Every click is a hit, even two clicks inside the same
  game tick (that happens a lot at 12+ CPS).
- **Your hit goes where you're aiming.** Java 1.8.9 aims the hit a little
  behind your mouse (the bug the MouseDelayFix mod fixes). Cindral uses your
  aim at that moment.
- **Loading chunks doesn't delay your hits.** The game, the network and the
  drawing each run on their own thread, and the game and network threads
  get the CPU first, so a busy frame doesn't hold back your clicks or the
  packets.

Things that are **the same as Java on purpose**: knockback, sprinting,
W-tap, S-tap, block-hitting, the 0.5 second wait after you miss a swing,
and reach (3 blocks). The server checks these. A client that changes them,
for example an automatic W-tap or no miss delay, gets flagged by
anticheats. In Cindral, W-tapping works the same way your hands already
know.

## Mods

Press **Right Shift** to open the mods menu. Every mod has its own
settings, and you can move and resize everything on the HUD.

**On screen:** FPS, CPS, Keystrokes, Ping, Coordinates, Direction HUD,
Reach Display, Combo Counter, Item Counter (pots, gapples, pearls,
arrows), Armor Status, Potion Effects, Clock, Memory.

**Gameplay and look:** Toggle Sprint, Zoom (C), 1.7 Animations,
Fullbright, No Hurt Cam, Hit Color, No Enchants, Motion Blur, Capes.

None of them play for you: nothing clicks, aims, reaches or moves on its
own.

Capes are on by default. To show them, Cindral asks OptiFine,
MinecraftCapes, LabyMod and Cosmetica for the capes of the players near
you, which means it sends those players' names to them. Turn the mod off
if you don't want that.

## Resource packs

Put packs in `%APPDATA%\cindral\resourcepacks` and turn them on in
**Options → Resource Packs**. PvP packs and server packs work, including
the OptiFine features most packs use: connected textures, custom items,
custom sky and colours, custom mob models, custom GUIs and fonts, and
animated textures. Mini items packs work, also with 1.7 Animations, and a
pack zipped inside its own folder loads too.

## Controls

| Key | |
|---|---|
| Right Shift | Mods menu and HUD editor |
| C | Zoom |
| F3 | Debug screen (F3 + A, B, D, G, H, T work like 1.8) |
| F2 | Screenshot |
| F5 | Change camera |

Change any key in **Options → Controls**.

## Questions

**Will I get banned?** Cindral sends the same packets as Minecraft 1.8.9,
so an anticheat sees a normal 1.8.9 player. Servers still make their own
rules about which clients they allow.

**Does it work with my Microsoft account?** Yes. You can add several
accounts and switch between them, or play offline.

**Where are my settings and logs?** In `%APPDATA%\cindral`. If the game
lags or stutters, send `logs\latest.log` from that folder when you report
it. It has a summary of the frame times every minute.

**Linux or macOS?** You can build it from source (below). Only Windows has
an installer for now.

## Building from source

Install [Rust](https://rustup.rs), then:

```sh
git clone https://github.com/aeromv/cindral.git
cd cindral
cargo build --release
```

The game is `target/release/cindral.exe` (`cindral` on Linux and macOS).
On Linux you need the ALSA headers first (`libasound2-dev` on Debian and
Ubuntu).

Useful launch options:

| Option | |
|---|---|
| `cindral play.example.net 25565` | Join a server straight away |
| `--setup-assets` | Download the Minecraft files and exit |
| `--render-scale 75` | Draw the world at 75 % of the window size; the HUD stays sharp |
| `--preset pvp` | Render distance 4, no FPS cap. Also `low`, `medium`, `high` |
| `--backend vulkan` | Use Vulkan instead of DirectX 12 |

Pull requests are welcome. Before you open one, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

MIT or Apache 2.0, your choice ([LICENSE-MIT](LICENSE-MIT),
[LICENSE-APACHE](LICENSE-APACHE)). The menu font is
[Inter](https://rsms.me/inter/) (SIL Open Font License 1.1).

Cindral is not an official Minecraft product and is not made by or
connected to Mojang or Microsoft.
