extern crate glutin;

use std::collections::{VecDeque, HashMap};
use glutin::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode};
use ::*;
use ::gfx::*;

pub struct InputMan {
    current_keys: HashMap<VirtualKeyCode, bool>,
    pressed_keys: HashMap<VirtualKeyCode, bool>,
    released_keys: HashMap<VirtualKeyCode, bool>
}

impl InputMan {
    pub fn new() -> InputMan {
        InputMan {
            current_keys: HashMap::new(),
            pressed_keys: HashMap::new(),
            released_keys: HashMap::new()
        }
    }
}

#[allow(dead_code)]
pub fn is_key_pressed(input_man: &InputMan, keycode: VirtualKeyCode) -> bool {
    *input_man.pressed_keys.get(&keycode).unwrap_or(&false)
}

#[allow(dead_code)]
pub fn is_key_released(input_man: &InputMan, keycode: VirtualKeyCode) -> bool {
    *input_man.released_keys.get(&keycode).unwrap_or(&false)
}

#[allow(dead_code)]
pub fn is_key_held(input_man: &InputMan, keycode: VirtualKeyCode) -> bool {
    *input_man.current_keys.get(&keycode).unwrap_or(&false)
}

pub fn process_events(window: &mut Window, input_man: &mut InputMan) {
    let mut events: VecDeque<Event> = VecDeque::new();
    window.events_loop.poll_events(|event| { events.push_back(event); });

    for event in events {
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Closed => { window.is_close_requested = true; },
                WindowEvent::Resized(w, h) => { println!("Resize to {}, {}", w, h); gfx::resize_window(window, w, h); },
                WindowEvent::KeyboardInput { input, .. } => { input::process_key_input(input_man, &input); },
                _ => ()
            },
            _ => ()
        }
    }
}

pub fn update_input(input_man: &mut InputMan) {
    input_man.pressed_keys.clear();
    input_man.released_keys.clear();
}

fn process_key_input(input_man: &mut InputMan, event: &KeyboardInput) {
    let keycode: VirtualKeyCode = event.virtual_keycode.unwrap();

    match event.state {
        ElementState::Pressed => {
            if !input::is_key_held(input_man, keycode)
            {
                input_man.pressed_keys.insert(keycode, true);
            }

            input_man.current_keys.insert(keycode, true);
        },
        ElementState::Released => {
            input_man.released_keys.insert(keycode, true);
            input_man.current_keys.insert(keycode, false);
        }
    }
}

// Refactor Idea:
// Hashmap of keycode to key state
// Modify key state when event is detected
// Pressed, Released, Held, None