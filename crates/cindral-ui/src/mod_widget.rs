// SPDX-License-Identifier: MIT OR Apache-2.0

//! The generic mod-widget layer of the HUD (ADR-0065, milestones M3 and M6).
//!
//! A built-in mod describes its widget as a few coloured text lines; the
//! client resolves the anchor and offset to a pixel position and hands this
//! layer a [`ModWidgetView`]. The renderer therefore knows nothing about
//! anchors or mods: it draws a translucent panel and the stacked lines, each
//! with the vanilla text shadow (`FontRenderer.drawStringWithShadow`: a
//! quarter-brightness copy one unit down and right).
//!
//! Sizes are in GUI units (the unit of `ScaledResolution`); the caller picks
//! the position from [`measure`] and `HudPlacement::rect`.
//!
//! The layer lives in this crate, not in the renderer, so the mods menu
//! (`menus::mods::hud_layer`) previews the widgets with exactly the code the HUD uses.

use crate::draw::{Color, DrawList, TextStyle};
use crate::geom::Rect;
use crate::text::FontAtlas;

/// The row pitch, `FontRenderer.FONT_HEIGHT`.
const LINE_HEIGHT: f32 = 9.0;
/// The panel colour: black at about 35 % opacity, like the chat band.
const PANEL: Color = [0.0, 0.0, 0.0, 0.35];

/// The shadow of the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowStyle {
    /// A quarter of the brightness of the text, `drawStringWithShadow`.
    #[default]
    Vanilla,
    /// One colour, `0xRRGGBB`.
    Color(u32),
    /// No shadow.
    Off,
}

/// How a widget looks around its text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetStyle {
    /// The panel behind the lines (straight-alpha RGBA), or `None` for none.
    pub background: Option<Color>,
    /// The corner radius of the panel, in GUI units.
    pub radius: f32,
    /// The margin between the panel edge and the text, in GUI units.
    pub padding: f32,
    /// The shadow of the text.
    pub shadow: ShadowStyle,
    /// Lines with an icon put the text first and the icon last, against the
    /// right edge (a widget on the right half of the screen).
    pub mirrored: bool,
}

impl Default for WidgetStyle {
    /// A black panel at 35 % with 2 units of margin and corner, vanilla shadow.
    fn default() -> Self {
        Self {
            background: Some(PANEL),
            radius: 2.0,
            padding: 2.0,
            shadow: ShadowStyle::Vanilla,
            mirrored: false,
        }
    }
}

/// The picture beside a line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineIcon {
    /// An item icon of the icon atlas (16 units), as [`DrawList::item`] draws it.
    Item {
        /// The client's icon key.
        key: u64,
        /// The enchantment glint.
        glint: bool,
    },
    /// A region of the GUI atlas (18 units), such as a status icon.
    Image {
        /// `[u_min, v_min, u_max, v_max]` in the GUI atlas.
        src: [f32; 4],
    },
}

impl LineIcon {
    /// The side of the square the icon fills, in GUI units.
    #[must_use]
    pub fn side(self) -> f32 {
        match self {
            Self::Item { .. } => ITEM_SIDE,
            Self::Image { .. } => IMAGE_SIDE,
        }
    }
}

/// The side of an item icon.
const ITEM_SIDE: f32 = 16.0;
/// The side of a status icon (`inventory.png` draws them 18 by 18).
const IMAGE_SIDE: f32 = 18.0;
/// The space between an icon and its text.
const ICON_GAP: f32 = 3.0;
/// The space between two rows that have icons.
const ROW_GAP: f32 = 2.0;

/// One line of a mod widget.
#[derive(Debug, Clone, PartialEq)]
pub struct ModLine {
    /// The text, drawn as is (no `\u{a7}` codes are interpreted).
    pub text: String,
    /// The colour as `0xRRGGBB`.
    pub color: u32,
    /// The picture beside the text.
    pub icon: Option<LineIcon>,
    /// A second row under the text, and its colour (the time of an effect).
    pub detail: Option<(String, u32)>,
}

impl ModLine {
    /// Creates a line of `text` in `color` (`0xRRGGBB`).
    #[must_use]
    pub fn new(text: impl Into<String>, color: u32) -> Self {
        Self {
            text: text.into(),
            color,
            icon: None,
            detail: None,
        }
    }

    /// The text rows of the line: one, or two with a detail.
    fn rows(&self) -> f32 {
        if self.detail.is_some() {
            2.0
        } else {
            1.0
        }
    }

    /// The width and the height of the line in the icon layout, at scale 1.
    fn icon_row_size(&self, font: &FontAtlas) -> (f32, f32) {
        let text = font.measure(&self.text, 1.0).max(
            self.detail
                .as_ref()
                .map_or(0.0, |(d, _)| font.measure(d, 1.0)),
        );
        let text_height = LINE_HEIGHT * self.rows() - 1.0;
        match self.icon {
            Some(icon) => {
                let gap = if text > 0.0 { ICON_GAP } else { 0.0 };
                (icon.side() + gap + text, icon.side().max(text_height))
            }
            None => (text, text_height),
        }
    }
}

/// Whether `lines` are drawn in the icon layout (one row per line, as tall as
/// its icon) rather than as plain text lines.
fn icon_layout(lines: &[ModLine]) -> bool {
    lines.iter().any(|l| l.icon.is_some() || l.detail.is_some())
}

/// One box of a widget drawn as a grid (Keystrokes).
#[derive(Debug, Clone, PartialEq)]
pub struct ModCell {
    /// `[x, y, width, height]` in GUI units from the widget's corner.
    pub rect: [f32; 4],
    /// The centred text.
    pub text: String,
    /// Lit: a light box with dark text.
    pub pressed: bool,
    /// The text colour while not pressed, `0xRRGGBB`.
    pub color: u32,
}

/// The box of a pressed key.
const PRESSED: Color = [1.0, 1.0, 1.0, 0.75];
/// The text of a pressed key.
const PRESSED_TEXT: Color = [0.0, 0.0, 0.0, 1.0];

/// A mod widget whose position is already resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct ModWidgetView {
    /// The lines, top to bottom; an empty list draws nothing.
    pub lines: Vec<ModLine>,
    /// Boxes drawn instead of the lines when not empty (Keystrokes).
    pub cells: Vec<ModCell>,
    /// The top-left corner of the panel in GUI units.
    pub position: [f32; 2],
    /// The widget scale, `1.0` for the vanilla size.
    pub scale: f32,
    /// The look of the panel and of the text shadow.
    pub style: WidgetStyle,
}

/// The size of the panel for `lines` at `scale`, in GUI units.
///
/// The result already includes `scale`, so it is what the placement needs to
/// resolve a rectangle against the screen; pass `1.0` for the unscaled size.
/// An empty list measures `(0.0, 0.0)`.
#[must_use]
pub fn measure(font: &FontAtlas, lines: &[ModLine], scale: f32, style: &WidgetStyle) -> (f32, f32) {
    if lines.is_empty() {
        return (0.0, 0.0);
    }
    if icon_layout(lines) {
        let rows: Vec<(f32, f32)> = lines.iter().map(|l| l.icon_row_size(font)).collect();
        let widest = rows.iter().map(|r| r.0).fold(0.0_f32, f32::max);
        let tall: f32 = rows.iter().map(|r| r.1).sum::<f32>() + ROW_GAP * (rows.len() as f32 - 1.0);
        let width = widest + 2.0 * style.padding;
        let height = tall + 2.0 * style.padding;
        return (width * scale, height * scale);
    }
    let widest = lines
        .iter()
        .map(|line| font.measure(&line.text, 1.0))
        .fold(0.0_f32, f32::max);
    let width = widest + 2.0 * style.padding;
    let height = LINE_HEIGHT * lines.len() as f32 + 2.0 * style.padding - 1.0;
    (width * scale, height * scale)
}

/// The size of a grid of `cells` at `scale`: the corner of the farthest box.
#[must_use]
pub fn measure_cells(cells: &[ModCell], scale: f32) -> (f32, f32) {
    let width = cells
        .iter()
        .map(|c| c.rect[0] + c.rect[2])
        .fold(0.0_f32, f32::max);
    let height = cells
        .iter()
        .map(|c| c.rect[1] + c.rect[3])
        .fold(0.0_f32, f32::max);
    (width * scale, height * scale)
}

impl ModWidgetView {
    /// The size the widget covers, in GUI units: its boxes, or its lines.
    #[must_use]
    pub fn size(&self, font: &FontAtlas) -> (f32, f32) {
        if self.cells.is_empty() {
            measure(font, &self.lines, self.scale, &self.style)
        } else {
            measure_cells(&self.cells, self.scale)
        }
    }
}

/// Draws every widget of `widgets` into `list`; `sf` is the GUI scale factor
/// (pixels per GUI unit).
pub fn draw_mod_widgets(list: &mut DrawList, widgets: &[ModWidgetView], sf: f32, font: &FontAtlas) {
    for widget in widgets {
        draw_widget(list, widget, sf, font);
    }
}

/// Draws the panel, then the lines with their shadows.
fn draw_widget(list: &mut DrawList, widget: &ModWidgetView, sf: f32, font: &FontAtlas) {
    if widget.scale <= 0.0 {
        return;
    }
    if !widget.cells.is_empty() {
        draw_cells(list, widget, sf, font);
        return;
    }
    if widget.lines.is_empty() {
        return;
    }
    let style = &widget.style;
    let (width, height) = measure(font, &widget.lines, widget.scale, style);
    let [x, y] = widget.position;
    if let Some(background) = style.background.filter(|colour| colour[3] > 0.0) {
        let panel = Rect::new(x * sf, y * sf, width * sf, height * sf);
        let radius = style.radius * widget.scale * sf;
        if radius > 0.0 {
            list.rounded_rect(panel, radius, background);
        } else {
            list.rect(panel, background);
        }
    }
    let unit = widget.scale * sf;
    let left = (x + style.padding * widget.scale) * sf;
    let top = (y + style.padding * widget.scale) * sf;
    if icon_layout(&widget.lines) {
        let right = left + width * sf - 2.0 * style.padding * unit;
        draw_icon_rows(list, widget, font, (left, right, top), unit);
        return;
    }
    for (row, line) in widget.lines.iter().enumerate() {
        let at = [left, top + LINE_HEIGHT * row as f32 * unit];
        shadowed_text(list, style, at, &line.text, line.color, unit);
    }
}

/// One run of text with the widget's shadow under it.
fn shadowed_text(
    list: &mut DrawList,
    style: &WidgetStyle,
    at: [f32; 2],
    text: &str,
    color: u32,
    unit: f32,
) {
    if text.is_empty() {
        return;
    }
    let ink = rgb(color);
    let shadow = match style.shadow {
        ShadowStyle::Vanilla => Some(shadow_of(ink)),
        ShadowStyle::Color(colour) => Some(rgb(colour)),
        ShadowStyle::Off => None,
    };
    if let Some(shadow) = shadow {
        list.text(
            [at[0] + unit, at[1] + unit],
            text,
            TextStyle::new(unit, shadow),
        );
    }
    list.text(at, text, TextStyle::new(unit, ink));
}

/// The icon layout: each line is a row as tall as its icon, the icon against
/// the near edge and the text (and its detail row) centred beside it.
/// Mirrored, the icon is against the right edge and the text right-aligned.
fn draw_icon_rows(
    list: &mut DrawList,
    widget: &ModWidgetView,
    font: &FontAtlas,
    (left, right, top): (f32, f32, f32),
    unit: f32,
) {
    let style = &widget.style;
    let mut y = top;
    for line in &widget.lines {
        let (_, row_height) = line.icon_row_size(font);
        let side = line.icon.map_or(0.0, LineIcon::side);
        if let Some(icon) = line.icon {
            let ix = if style.mirrored {
                right - side * unit
            } else {
                left
            };
            let iy = y + (row_height - side) * 0.5 * unit;
            let dst = Rect::new(ix, iy, side * unit, side * unit);
            match icon {
                LineIcon::Item { key, glint } => list.item(key, dst, glint),
                LineIcon::Image { src } => list.image(src, dst, [1.0, 1.0, 1.0, 1.0]),
            }
        }
        let offset = if line.icon.is_some() {
            (side + ICON_GAP) * unit
        } else {
            0.0
        };
        let text_height = LINE_HEIGHT * line.rows() - 1.0;
        let mut ty = y + (row_height - text_height) * 0.5 * unit;
        let detail = line
            .detail
            .as_ref()
            .map(|(text, color)| (text.as_str(), *color));
        for (text, color) in std::iter::once((line.text.as_str(), line.color)).chain(detail) {
            let tx = if style.mirrored {
                right - offset - font.measure(text, unit)
            } else {
                left + offset
            };
            shadowed_text(list, style, [tx, ty], text, color, unit);
            ty += LINE_HEIGHT * unit;
        }
        y += (row_height + ROW_GAP) * unit;
    }
}

/// Draws the boxes of a grid widget, each with its text centred: a pressed box
/// is lit with dark text, as the Keystrokes of PvP clients do.
fn draw_cells(list: &mut DrawList, widget: &ModWidgetView, sf: f32, font: &FontAtlas) {
    let style = &widget.style;
    let unit = widget.scale * sf;
    let [x, y] = widget.position;
    for cell in &widget.cells {
        let [cx, cy, cw, ch] = cell.rect;
        let rect = Rect::new(
            (x + cx * widget.scale) * sf,
            (y + cy * widget.scale) * sf,
            cw * unit,
            ch * unit,
        );
        let fill = if cell.pressed {
            Some(PRESSED)
        } else {
            style.background
        };
        if let Some(fill) = fill.filter(|colour| colour[3] > 0.0) {
            let radius = style.radius * unit;
            if radius > 0.0 {
                list.rounded_rect(rect, radius, fill);
            } else {
                list.rect(rect, fill);
            }
        }
        if cell.text.is_empty() {
            continue;
        }
        let text_width = font.measure(&cell.text, 1.0) * unit;
        let at = [
            rect.x + (rect.width - text_width) * 0.5,
            rect.y + (rect.height - (LINE_HEIGHT - 1.0) * unit) * 0.5,
        ];
        let ink = if cell.pressed {
            PRESSED_TEXT
        } else {
            rgb(cell.color)
        };
        let shadow = match style.shadow {
            _ if cell.pressed => None,
            ShadowStyle::Vanilla => Some(shadow_of(ink)),
            ShadowStyle::Color(colour) => Some(rgb(colour)),
            ShadowStyle::Off => None,
        };
        if let Some(shadow) = shadow {
            list.text(
                [at[0] + unit, at[1] + unit],
                &cell.text,
                TextStyle::new(unit, shadow),
            );
        }
        list.text(at, &cell.text, TextStyle::new(unit, ink));
    }
}

/// Converts `0xRRGGBB` to an opaque colour.
fn rgb(value: u32) -> Color {
    let channel = |shift: u32| ((value >> shift) & 0xFF) as f32 / 255.0;
    [channel(16), channel(8), channel(0), 1.0]
}

/// The shadow colour: a quarter of the brightness.
fn shadow_of([r, g, b, a]: Color) -> Color {
    [r / 4.0, g / 4.0, b / 4.0, a]
}

#[cfg(test)]
mod tests {
    use crate::draw::DrawCommand;

    use super::*;

    fn widget(scale: f32) -> ModWidgetView {
        ModWidgetView {
            cells: Vec::new(),
            lines: vec![
                ModLine::new("FPS: 60", 0xFF8000),
                ModLine::new("x", 0xFFFFFF),
            ],
            position: [10.0, 20.0],
            scale,
            style: WidgetStyle::default(),
        }
    }

    #[test]
    fn empty_widgets_measure_zero_and_draw_nothing() {
        let font = FontAtlas::fallback();
        assert_eq!(
            measure(&font, &[], 2.0, &WidgetStyle::default()),
            (0.0, 0.0)
        );
        let mut list = DrawList::new();
        let empty = ModWidgetView {
            cells: Vec::new(),
            lines: Vec::new(),
            ..widget(1.0)
        };
        draw_mod_widgets(&mut list, &[empty], 2.0, &font);
        assert!(list.is_empty());
    }

    #[test]
    fn size_follows_the_widest_line_and_the_row_count() {
        let font = FontAtlas::fallback();
        let w = widget(1.0);
        let style = WidgetStyle::default();
        let (width, height) = measure(&font, &w.lines, 1.0, &style);
        assert_eq!(width, font.measure("FPS: 60", 1.0) + 2.0 * style.padding);
        assert_eq!(height, 2.0 * LINE_HEIGHT + 2.0 * style.padding - 1.0);
        let (sw, sh) = measure(&font, &w.lines, 1.5, &style);
        assert_eq!((sw, sh), (width * 1.5, height * 1.5));
    }

    #[test]
    fn a_widget_emits_a_panel_then_a_shadow_and_text_per_line() {
        let font = FontAtlas::fallback();
        let mut list = DrawList::new();
        draw_mod_widgets(&mut list, &[widget(2.0)], 3.0, &font);
        let commands = list.commands();
        assert_eq!(commands.len(), 1 + 2 * 2);
        let (width, height) = measure(&font, &widget(2.0).lines, 2.0, &WidgetStyle::default());
        match &commands[0] {
            DrawCommand::RoundedRect { rect, .. } => {
                assert_eq!(*rect, Rect::new(30.0, 60.0, width * 3.0, height * 3.0));
            }
            other => panic!("expected the panel, got {other:?}"),
        }
        let (
            DrawCommand::Text {
                position: shadow,
                style: s,
                ..
            },
            DrawCommand::Text {
                position: text,
                style: t,
                text: body,
            },
        ) = (&commands[1], &commands[2])
        else {
            panic!("expected shadow then text");
        };
        assert_eq!(body, "FPS: 60");
        assert_eq!(t.scale, 6.0);
        assert_eq!([shadow[0] - text[0], shadow[1] - text[1]], [6.0, 6.0]);
        assert_eq!(text[0], (10.0 + 4.0) * 3.0);
        assert_eq!(t.color, [1.0, 128.0 / 255.0, 0.0, 1.0]);
        assert_eq!(s.color[0], t.color[0] / 4.0);
        let DrawCommand::Text {
            position: second, ..
        } = &commands[4]
        else {
            panic!("expected the second line");
        };
        assert_eq!(second[1] - text[1], LINE_HEIGHT * 6.0);
    }

    #[test]
    fn the_padding_changes_the_size_and_where_the_text_starts() {
        let font = FontAtlas::fallback();
        let mut w = widget(1.0);
        w.style.padding = 5.0;
        let (width, _) = measure(&font, &w.lines, 1.0, &w.style);
        assert_eq!(width, font.measure("FPS: 60", 1.0) + 10.0);
        let mut list = DrawList::new();
        draw_mod_widgets(&mut list, &[w], 1.0, &font);
        let DrawCommand::Text { position, .. } = &list.commands()[2] else {
            panic!("expected the text");
        };
        assert_eq!(position[0], 10.0 + 5.0);
    }

    #[test]
    fn no_background_draws_no_panel_and_no_shadow_draws_no_copy() {
        let font = FontAtlas::fallback();
        let mut w = widget(1.0);
        w.style.background = None;
        w.style.shadow = ShadowStyle::Off;
        let mut list = DrawList::new();
        draw_mod_widgets(&mut list, &[w], 1.0, &font);
        assert_eq!(list.commands().len(), 2, "one text per line");
        assert!(list
            .commands()
            .iter()
            .all(|command| matches!(command, DrawCommand::Text { .. })));
    }

    #[test]
    fn a_custom_shadow_has_its_own_colour_and_no_radius_gives_square_corners() {
        let font = FontAtlas::fallback();
        let mut w = widget(1.0);
        w.style.shadow = ShadowStyle::Color(0x00_00FF);
        w.style.radius = 0.0;
        let mut list = DrawList::new();
        draw_mod_widgets(&mut list, &[w], 1.0, &font);
        assert!(matches!(list.commands()[0], DrawCommand::Rect { .. }));
        let DrawCommand::Text { style, .. } = &list.commands()[1] else {
            panic!("expected the shadow");
        };
        assert_eq!(style.color, [0.0, 0.0, 1.0, 1.0]);
    }
}
