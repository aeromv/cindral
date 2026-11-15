// SPDX-License-Identifier: MIT OR Apache-2.0
//! The read-only snapshot of the game a mod sees each frame.

/// Facts about the running game, built once per frame by the client and handed to
/// every enabled mod. It is `Copy` so building and passing it never allocates.
///
/// Mods read this instead of reaching into the client, which keeps them testable
/// and keeps the client free of any knowledge of a particular mod.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ModContext {
    /// Monotonic time in milliseconds, for rates such as clicks per second.
    pub now_ms: u64,
    /// Frames drawn during the last second.
    pub fps: u32,
    /// Smoothed duration of a frame, in milliseconds.
    pub frame_ms: f32,
    /// Position of the player's feet, in blocks.
    pub position: [f64; 3],
    /// Horizontal view angle, in degrees.
    pub yaw: f32,
    /// Vertical view angle, in degrees.
    pub pitch: f32,
    /// Whether the player is sprinting.
    pub sprinting: bool,
    /// Left clicks since the previous frame.
    pub left_clicks: u32,
    /// Right clicks since the previous frame.
    pub right_clicks: u32,
    /// Round-trip time to the server, if connected to one and measured.
    pub ping_ms: Option<u32>,
    /// The movement keys and mouse buttons held (Keystrokes).
    pub keys: KeyState,
    /// The worn armour, helmet first (Armor Status).
    pub armor: [Option<ArmorPiece>; 4],
    /// The item in the selected hotbar slot (Armor Status, "held item").
    pub held: Option<ArmorPiece>,
    /// The active potion effects, in effect id order (Potion Effects).
    pub effects: [Option<EffectInfo>; MAX_EFFECTS],
    /// The fights so far (Reach Display, Combo Counter).
    pub combat: CombatInfo,
}

/// The local player's fights, counted from the start of the session; a mod
/// compares them with what it saw last frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CombatInfo {
    /// Hits the server confirmed on an entity the player attacked.
    pub hits: u32,
    /// The distance of the last attack, eye to the point hit, in hundredths
    /// of a block.
    pub last_reach_cm: Option<u16>,
    /// Times the player was hurt.
    pub hurts: u32,
}

/// The most effects a mod sees; 1.8 has 23 kinds, more than a few at once is rare.
pub const MAX_EFFECTS: usize = 8;

/// The keys of the Keystrokes mod, held or not this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyState {
    /// Walk forward (W).
    pub forward: bool,
    /// Strafe left (A).
    pub left: bool,
    /// Walk back (S).
    pub back: bool,
    /// Strafe right (D).
    pub right: bool,
    /// Jump (Space).
    pub jump: bool,
    /// Sneak (Shift).
    pub sneak: bool,
    /// Attack (left mouse button).
    pub attack: bool,
    /// Use item (right mouse button).
    pub use_item: bool,
}

/// One worn piece of armour, or the held item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmorPiece {
    /// The registry name of the item, such as `diamond_helmet`.
    pub name: &'static str,
    /// The damage taken so far.
    pub damage: i16,
    /// The damage at which it breaks; `0` for an item that does not wear.
    pub max_damage: u16,
}

/// One active potion effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectInfo {
    /// `Potion.id` (1 Speed to 23 Saturation).
    pub id: u8,
    /// The level minus one.
    pub amplifier: i8,
    /// Remaining duration, in ticks (20 per second).
    pub ticks: i32,
}
