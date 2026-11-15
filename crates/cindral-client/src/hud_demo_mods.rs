// SPDX-License-Identifier: MIT OR Apache-2.0

//! The scene of the `mods` and `mods_list` views of the UI demo (ADR-0065): the
//! built-in mods with FPS, coordinates and CPS on and spread over the screen, so
//! the capture shows the HUD widgets around the open panel. `mods_list` is the
//! same scene under its former name.

use cindral_builtin_mods::{
    mods, Anchor, ArmorPiece, EffectInfo, HudPlacement, KeyState, ModContext, ModRegistry,
    MAX_EFFECTS,
};
use cindral_core::lang::Lang;
use cindral_ui::menus::ModsView;
use cindral_ui::FontAtlas;

use crate::item_icons::{slot_item, ItemView};
use crate::mods_hud_view::WidgetIcons;
use crate::{hud_editor_view, mods_view};

/// The armour the demo player wears, helmet first: item id, damage.
const ARMOR: [(i16, i16); 4] = [(310, 20), (311, 300), (308, 190), (313, 5)];
/// The sword in the demo player's hand: item id, damage.
const HELD: (i16, i16) = (276, 61);

/// The mods menu of the demo: every built-in mod listed, three HUD mods on at
/// their own corner of the screen.
#[must_use]
pub fn view(
    lang: &Lang,
    font: &FontAtlas,
    display: (u32, u32),
    (items, atlas): (ItemView<'_>, Option<&cindral_assets::GuiAtlas>),
) -> ModsView {
    let stack = |(item_id, damage): (i16, i16)| cindral_protocol::packets::play::ItemStack {
        item_id,
        stack_size: 1,
        damage,
        nbt: None,
    };
    let icons = WidgetIcons {
        armor: ARMOR.map(|piece| Some(slot_item(items, &stack(piece)))),
        held: Some(slot_item(items, &stack(HELD))),
        ..WidgetIcons::default()
    }
    .with_atlas(atlas);
    let mut registry = match ModRegistry::new(mods::all()) {
        Ok(registry) => registry,
        Err(error) => {
            tracing::error!(%error, "no mods to show in the demo");
            return ModsView::default();
        }
    };
    let places = [
        ("fps", Anchor::TopLeft, 1.0),
        ("coordinates", Anchor::BottomLeft, 1.0),
        ("cps", Anchor::MiddleRight, 1.5),
        ("keystrokes", Anchor::MiddleLeft, 1.0),
        ("armor_status", Anchor::BottomRight, 1.0),
        ("potion_effects", Anchor::TopRight, 1.0),
    ];
    for (id, anchor, scale) in places {
        let placement = HudPlacement {
            scale,
            ..HudPlacement::at(anchor, 0.0, 0.0)
        };
        // The ids are the built-in ones, so these cannot fail.
        let _ = registry.set_enabled(id, true);
        let _ = registry.set_placement(id, placement);
    }
    let piece = |(item_id, damage): (i16, i16)| {
        let item = cindral_game::items::lookup(i32::from(item_id))?;
        Some(ArmorPiece {
            name: item.name,
            damage,
            max_damage: item.max_damage,
        })
    };
    let mut effects = [None; MAX_EFFECTS];
    effects[0] = Some(EffectInfo {
        id: 1,
        amplifier: 1,
        ticks: 20 * 95,
    });
    effects[1] = Some(EffectInfo {
        id: 12,
        amplifier: 0,
        ticks: 20 * 180,
    });
    effects[2] = Some(EffectInfo {
        id: 5,
        amplifier: 1,
        ticks: 20 * 7,
    });
    registry.update(&ModContext {
        fps: 144,
        position: [123.4, 64.0, -56.7],
        keys: KeyState {
            forward: true,
            attack: true,
            ..KeyState::default()
        },
        armor: ARMOR.map(piece),
        held: piece(HELD),
        effects,
        ..ModContext::default()
    });
    ModsView {
        hud: hud_editor_view::build(&registry, lang, &icons, font, display),
        ..mods_view::build(&registry, lang)
    }
}
