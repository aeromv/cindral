// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests of the list page of the mods menu: the cards, the search and the tabs.

use super::*;

#[test]
fn the_cards_name_every_mod_and_the_tabs_name_the_categories() {
    let names = texts(&mut on_list(), &view());
    for wanted in ["FPS", "Coordinates", "Toggle Sprint", "HUD", "Gameplay"] {
        assert!(has_text(&names, wanted), "{wanted} in {names:?}");
    }
}

#[test]
fn the_switch_of_a_card_asks_for_the_other_state() {
    let (switch, _) = controls(first_card(), mods_layout(VIEWPORT).scale);
    let outcome = show(&mut on_list(), click_at(switch));
    assert_eq!(
        outcome,
        Some(ModsOutcome::Toggle {
            id: "fps".into(),
            enabled: true
        })
    );
    let mut on = view();
    on.cards[0].enabled = true;
    let outcome = show_view(&mut on_list(), &on, click_at(switch));
    assert_eq!(
        outcome,
        Some(ModsOutcome::Toggle {
            id: "fps".into(),
            enabled: false
        })
    );
}

#[test]
fn the_state_bar_of_a_card_says_whether_the_mod_is_on() {
    let names = texts(&mut on_list(), &view());
    assert!(has_text(&names, "DISABLED"), "{names:?}");
    let mut on = view();
    on.cards[0].enabled = true;
    let names = texts(&mut on_list(), &on);
    assert!(has_text(&names, "ENABLED"), "{names:?}");
}

#[test]
fn a_click_beside_everything_does_nothing() {
    let away = Rect::new(VIEWPORT.x + 4.0, VIEWPORT.y + 4.0, 4.0, 4.0);
    assert_eq!(show(&mut on_list(), click_at(away)), None);
}

#[test]
fn typing_filters_the_cards_by_name() {
    let mut screen = on_list();
    let typed = UiInput::new(vec![InputEvent::Text("sprint".into())]);
    show(&mut screen, typed);
    let names = texts(&mut screen, &view());
    assert!(has_text(&names, "Toggle Sprint"));
    assert!(!has_text(&names, "FPS") && !has_text(&names, "Coordinates"));
    let erase = vec![InputEvent::KeyPressed(Key::Backspace); 6];
    show(&mut screen, UiInput::new(erase));
    assert!(has_text(&texts(&mut screen, &view()), "FPS"));
}

#[test]
fn a_search_without_result_says_so() {
    let mut screen = on_list();
    show(
        &mut screen,
        UiInput::new(vec![InputEvent::Text("zzz".into())]),
    );
    let names = texts(&mut screen, &view());
    assert!(has_text(&names, "cindral.mods.empty"), "{names:?}");
}

#[test]
fn a_category_in_the_sidebar_keeps_only_its_mods() {
    let layout = mods_layout(VIEWPORT);
    let mut screen = on_list();
    // The items are: All, HUD, Gameplay.
    let gameplay = item_rect(layout.items, layout.scale, 2);
    show(&mut screen, click_at(gameplay));
    let names = texts(&mut screen, &view());
    assert!(has_text(&names, "Toggle Sprint"));
    assert!(!has_text(&names, "FPS"));
}

#[test]
fn the_enabled_filter_keeps_the_mods_that_are_on() {
    let mut screen = on_list();
    let mut on = view();
    on.cards[1].enabled = true;
    let enabled = mods_layout(VIEWPORT).filter;
    // The second half of the control is "Enabled".
    let right = Rect::new(
        enabled.x + enabled.width * 0.75,
        enabled.y,
        4.0,
        enabled.height,
    );
    show_view(&mut screen, &on, click_at(right));
    let names = texts(&mut screen, &on);
    assert!(has_text(&names, "Coordinates"));
    assert!(!has_text(&names, "FPS"));
}

#[test]
fn the_header_counts_the_mods_that_are_on() {
    let mut on = view();
    on.cards[0].enabled = true;
    on.cards[2].enabled = true;
    let names = texts(&mut on_list(), &on);
    assert!(has_text(&names, "2 cindral.mods.enabled"), "{names:?}");
}

#[test]
fn the_pointer_is_not_needed_to_type() {
    // Typing works without any click on the search box first.
    let mut screen = on_list();
    let events = vec![
        InputEvent::PointerMoved {
            position: [0.0, 0.0],
        },
        InputEvent::Text("c".into()),
        InputEvent::PointerReleased {
            position: [0.0, 0.0],
            button: PointerButton::Left,
        },
    ];
    show(&mut screen, UiInput::new(events));
    let names = texts(&mut screen, &view());
    assert!(has_text(&names, "Coordinates") && !has_text(&names, "FPS"));
}

/// A view with enough mods for the list to scroll.
fn many_view() -> ModsView {
    let mut many = view();
    many.cards = (0..15)
        .map(|n| card(&format!("mod{n}"), &format!("Mod {n}"), 0, Vec::new()))
        .collect();
    many
}

#[test]
fn a_press_on_the_scrollbar_track_scrolls_the_list() {
    let many = many_view();
    let layout = mods_layout(VIEWPORT);
    let mut screen = on_list();
    show_view(&mut screen, &many, UiInput::default());
    let x = layout.list.right() + 3.0 * layout.scale;
    let low = [x, layout.list.bottom() - 2.0];
    show_view(
        &mut screen,
        &many,
        UiInput::new(vec![InputEvent::PointerPressed {
            position: low,
            button: PointerButton::Left,
        }]),
    );
    assert!(
        screen.scroll > 0.0,
        "the press jumped down: {}",
        screen.scroll
    );
    show_view(
        &mut screen,
        &many,
        UiInput::new(vec![InputEvent::PointerReleased {
            position: low,
            button: PointerButton::Left,
        }]),
    );
    assert!(!screen.bar.is_active());
}
