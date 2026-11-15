// SPDX-License-Identifier: MIT OR Apache-2.0
//! The text a HUD mod shows, in buffers that are reused from frame to frame.

use super::HudStyle;

/// A picture drawn at the start of a line; the client turns it into a real
/// icon (an item, a status icon of `inventory.png`) when it has the textures,
/// and the line keeps its text either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HudIcon {
    /// No picture.
    #[default]
    None,
    /// The worn armour piece in this slot, `0` the helmet to `3` the boots.
    Armor(u8),
    /// The item in the selected hotbar slot.
    Held,
    /// The status icon of the potion effect with this id.
    Effect(u8),
}

/// One line of a widget.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HudLine {
    /// The text.
    pub text: String,
    /// Color as `0xRRGGBB`.
    pub color: u32,
    /// The picture beside the text.
    pub icon: HudIcon,
    /// A second, smaller row of text under `text` beside the icon (the time
    /// left of an effect); empty for none.
    pub detail: String,
    /// The colour of `detail`.
    pub detail_color: u32,
}

/// A box of a widget drawn as a grid (Keystrokes): a rectangle in GUI units
/// from the widget's top-left corner, with centred text, lit while pressed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HudCell {
    /// `[x, y, width, height]` in GUI units.
    pub rect: [f32; 4],
    /// The centred text.
    pub text: String,
    /// Drawn lit (light box, dark text) instead of the widget's background.
    pub pressed: bool,
    /// The text colour while not pressed, `0xRRGGBB`.
    pub color: u32,
}

/// The lines (or the boxes) of one widget.
///
/// `clear` keeps the strings allocated, so after the first frames a mod writes
/// its lines without touching the allocator (`write!` into [`line`](Self::line)).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudContent {
    lines: Vec<HudLine>,
    used: usize,
    cells: Vec<HudCell>,
    used_cells: usize,
    /// The look of the widget, set by the registry from the shared options.
    pub style: HudStyle,
}

impl HudContent {
    /// Forgets the lines but keeps their buffers.
    pub fn clear(&mut self) {
        self.used = 0;
        self.used_cells = 0;
    }

    /// Adds a box at `rect` (see [`HudCell`]) and returns its text to fill.
    pub fn cell(&mut self, rect: [f32; 4], pressed: bool, color: u32) -> &mut String {
        if self.used_cells == self.cells.len() {
            self.cells.push(HudCell::default());
        }
        let cell = &mut self.cells[self.used_cells];
        self.used_cells += 1;
        cell.rect = rect;
        cell.pressed = pressed;
        cell.color = color;
        cell.text.clear();
        &mut cell.text
    }

    /// The boxes of this frame, in the order they were added.
    #[must_use]
    pub fn cells(&self) -> &[HudCell] {
        &self.cells[..self.used_cells]
    }

    /// Whether the widget has nothing to draw this frame.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.used == 0 && self.used_cells == 0
    }

    /// Starts a new line of the given color and returns its (empty) text to write into.
    pub fn line(&mut self, color: u32) -> &mut String {
        if self.used == self.lines.len() {
            self.lines.push(HudLine::default());
        }
        let line = &mut self.lines[self.used];
        self.used += 1;
        line.text.clear();
        line.color = color;
        line.icon = HudIcon::None;
        line.detail.clear();
        &mut line.text
    }

    /// Adds a line with `icon` beside it and returns it, so the mod can write
    /// both its text and its `detail`.
    pub fn icon_line(&mut self, icon: HudIcon, color: u32) -> &mut HudLine {
        self.line(color);
        let line = &mut self.lines[self.used - 1];
        line.icon = icon;
        line
    }

    /// The lines written since the last [`clear`](Self::clear).
    #[must_use]
    pub fn lines(&self) -> &[HudLine] {
        &self.lines[..self.used]
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use super::*;

    #[test]
    fn lines_are_kept_in_order() {
        let mut c = HudContent::default();
        c.line(0xff0000).push('a');
        write!(c.line(0x00ff00), "b{}", 2).expect("string write");
        let texts: Vec<_> = c
            .lines()
            .iter()
            .map(|l| (l.text.as_str(), l.color))
            .collect();
        assert_eq!(texts, [("a", 0xff0000), ("b2", 0x00ff00)]);
    }

    #[test]
    fn clear_hides_old_lines_and_reuses_buffers() {
        let mut c = HudContent::default();
        c.line(1).push_str("long enough to allocate");
        let capacity = c.lines[0].text.capacity();
        c.clear();
        assert!(c.lines().is_empty());
        c.line(2).push('x');
        assert_eq!(c.lines().len(), 1);
        assert_eq!(c.lines[0].text.capacity(), capacity);
        assert_eq!(c.lines()[0].text, "x", "the old text is gone");
    }
}
