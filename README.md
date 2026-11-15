# Cindral

A fast Minecraft 1.8.x client written in Rust, aimed at PvP on low-end PCs.

Cindral speaks protocol 47, so it joins 1.8.8 and 1.8.9 servers like a
vanilla client. It is a fork of [Umicraft](https://github.com/MATHIP6/umicraft)
with work focused on frame rate, input latency and a PvP mod set.

Cindral is not affiliated with Mojang Studios or Microsoft, and it ships no
Mojang assets: the official 1.8.8 files are downloaded on your machine the
first time you run it.

## Why another client

Most of the work here was measured on a Pentium Gold G5400 with Intel UHD 610
graphics, the kind of PC where vanilla 1.8.9 runs at about 40 fps. Some of the
changes:

- **Frame pacing.** The game ticks start on time on Windows. The default
  15.6 ms timer made ticks start 8 ms late on average, which made movement
  and knockback feel heavy at any frame rate. With a 1 ms timer they start
  about 1 ms late.
- **Rendering.** Entities and their held items are culled like vanilla
  (view frustum, size-based range), name tags are drawn in a single call, the
  sky is drawn after the terrain, and two frames stay in flight when vsync is
  off. Every change was checked against a screenshot of the same frame.
- **Networking.** Packets never stall the network thread, so keep-alives are
  answered at once and your clicks are never dropped.
- **Correctness.** The scoreboard survives a dimension change, long chat
  messages no longer disconnect you, and a server-side respawn gives the
  mouse back, as in vanilla.

Measured numbers and how they were taken are in
[`docs/perf-log.md`](docs/perf-log.md). Results depend a lot on the server and
on what else is running: close your browser before judging the frame rate.

## Getting started

You need a stable Rust toolchain to build it.

Download the official assets once (about 110 MB, SHA-1 checked):

```sh
cargo run --release -p umicraft-client -- --setup-assets
```

Then start the client:

```sh
cargo run --release -p umicraft-client -- --preset pvp
```

The window opens on the title screen; add your server under Multiplayer. You
can also connect directly with `cindral <host> <port>`.

Presets, picked automatically on the first start from your GPU:

| Preset | For |
|---|---|
| `low` | no graphics card, or a very weak one |
| `medium` | integrated graphics |
| `high` | a graphics card |
| `pvp` | sharp picture, uncapped frame rate, render distance 4 |

Always build with `--release`: a debug build is several times slower.

## Mods

Press **Right Shift** in game. Everything runs inside the client, no extra
downloads:

FPS, CPS, Keystrokes, Armor Status, Potion Effects, Coordinates, Ping,
Toggle Sprint (with the `[Sprinting (Toggled)]` indicator), Reach Display,
Combo Counter, Zoom, Fullbright, No Hurt Cam and Motion Blur.

Reach Display and Combo Counter only count hits the server confirmed, so
clicking during the target's invulnerability does not inflate them.

There is no autoclicker and there will not be one.

## Measuring on your own machine

```sh
cindral --bench-scene --width 1920 --height 1080 --preset pvp --bench-in-flight 2
cindral --preset pvp --perf-log perf.txt
```

The first renders a fixed scene and prints the frame, CPU and GPU times. The
second writes one line per second while you play: frame rate, CPU time per
stage, GPU time per pass, how long packets wait, how late the ticks start and
how often the server corrected your position.

## Project layout

The crates keep the `umicraft-*` names of the upstream project, which makes
its fixes easy to bring over.

| Crate | What it does |
|---|---|
| `umicraft-protocol` | Protocol 47 packets, framing, compression, encryption |
| `umicraft-net` | Connection, login and the play session |
| `umicraft-assets` | Asset download and cache, the client pack |
| `umicraft-model` | 1.8 block states and models |
| `umicraft-world` | Chunk storage and world updates |
| `umicraft-mesh` | Chunk meshing |
| `umicraft-render` | The wgpu renderer |
| `umicraft-ui` | The UI toolkit |
| `umicraft-game` | Game logic: movement, entities, inventory, chat, HUD state |
| `umicraft-audio` | Sound |
| `umicraft-auth` | Offline, launcher and Microsoft sessions |
| `umicraft-builtin-mods` | The built-in mods |
| `umicraft-client` | The client binary (`cindral`) |
| `umicraft-testkit` | Fake server and test tools |

Before sending a change:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Design notes are in [`docs/decisions`](docs/decisions) and the vanilla
parity status in [`docs/parity.md`](docs/parity.md).

## Credits

- [Umicraft](https://github.com/MATHIP6/umicraft) by its contributors, the
  project Cindral is built on.
- [Inter](https://rsms.me/inter/) by The Inter Project Authors (SIL OFL 1.1),
  used for the title screen.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT), at your option.
