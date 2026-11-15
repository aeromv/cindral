// SPDX-License-Identifier: MIT OR Apache-2.0

//! Design tokens of the modern widgets.
//!
//! Lengths and text sizes are in GUI units; colours are straight-alpha RGBA.
//! The surfaces stay translucent so the panorama shows through.

use crate::draw::Color;

/// Pixel size the brand face is rasterized at; text scales are
/// `wanted_px / BRAND_PIXEL_SIZE`.
pub use crate::brand_font::BRAND_PIXEL_SIZE;

/// Fill of buttons and panels at rest: warm charcoal glass.
pub const SURFACE: Color = [0.12, 0.115, 0.12, 0.66];
/// Fill of a hovered button.
pub const SURFACE_HOVER: Color = [0.20, 0.17, 0.155, 0.88];
/// Fill of a disabled button.
pub const SURFACE_DISABLED: Color = [0.06, 0.06, 0.07, 0.32];
/// The highlight colour of the brand: ember orange.
pub const ACCENT: Color = [1.00, 0.45, 0.16, 1.0];
/// The accent under the pointer or while dragged.
pub const ACCENT_BRIGHT: Color = [1.00, 0.58, 0.30, 1.0];
/// Primary text.
pub const TEXT: Color = [0.97, 0.96, 0.95, 1.0];
/// Secondary text.
pub const TEXT_MUTED: Color = [0.66, 0.64, 0.62, 1.0];
/// The darkest point of the screen vignette (alpha is the strength).
pub const VIGNETTE: Color = [0.01, 0.01, 0.015, 0.62];
/// The menu background with no world or panorama behind it, top and bottom.
pub const BACKDROP_TOP: Color = [0.075, 0.072, 0.078, 1.0];
/// See [`BACKDROP_TOP`].
pub const BACKDROP_BOTTOM: Color = [0.030, 0.028, 0.032, 1.0];
/// Factor applied to the alpha of text and icons on a disabled widget.
pub const DISABLED_OPACITY: f32 = 0.4;

/// Corner radius of buttons and panels.
pub const RADIUS: f32 = 2.5;
/// The gap between neighbouring widgets and between an icon and its label.
pub const SPACING: f32 = 4.0;
/// Height of a standard button.
pub const BUTTON_HEIGHT: f32 = 16.0;

/// Size of button labels.
pub const TEXT_SIZE_BODY: f32 = 7.0;
/// Size of small captions.
pub const TEXT_SIZE_SMALL: f32 = 5.5;
/// Size of headings.
pub const TEXT_SIZE_TITLE: f32 = 16.0;

/// Fraction of the hover fade gained or lost per 20 TPS tick (about 200 ms).
pub const HOVER_FADE_STEP: f32 = 0.25;

/// Linear blend of two colours, `t` clamped to `0.0..=1.0`.
#[must_use]
pub fn mix(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| from[i] * (1.0 - t) + to[i] * t)
}

/// `color` with its alpha multiplied by `opacity`.
#[must_use]
pub fn with_opacity(color: Color, opacity: f32) -> Color {
    [color[0], color[1], color[2], color[3] * opacity]
}
