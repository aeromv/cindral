// SPDX-License-Identifier: MIT OR Apache-2.0

//! Tests of the window glue.

use winit::event::ElementState;
use winit::keyboard::KeyCode;

use super::keyboard::*;
use super::redraw::*;
use crate::options::{self};

#[test]
fn the_frame_rate_stays_the_same_when_the_window_loses_the_focus() {
    assert_eq!(
        frame_cap(options::UNLIMITED_FPS, true),
        options::UNLIMITED_FPS
    );
    assert_eq!(frame_cap(144, true), 144);
    assert_eq!(
        frame_cap(options::UNLIMITED_FPS, false),
        options::UNLIMITED_FPS
    );
    assert_eq!(frame_cap(20, false), 20);
}

#[test]
fn a_fresh_f2_press_is_the_only_toggle() {
    assert!(is_debug_toggle(KeyCode::F4, ElementState::Pressed, false));
    assert!(!is_debug_toggle(KeyCode::F4, ElementState::Pressed, true));
    assert!(!is_debug_toggle(KeyCode::F4, ElementState::Released, false));
    assert!(!is_debug_toggle(KeyCode::F3, ElementState::Pressed, false));
    assert!(!is_debug_toggle(KeyCode::F2, ElementState::Pressed, false));
    assert!(!is_debug_toggle(KeyCode::F1, ElementState::Pressed, false));
}
