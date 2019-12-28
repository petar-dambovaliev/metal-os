use crate::time::duration_now;
use alloc::{vec, vec::Vec};
use core::ops::Sub;
use core::time::Duration;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    pub static ref MOUSE: Mutex<MouseInternal> = { Mutex::new(MouseInternal::new()) };
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
enum Code {
    Left = 0x01,
    Right = 0x02,
    Middle = 0x04,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum State {
    Click,
    Release,
}

impl State {
    pub fn get(byte: u8, code: Code) -> State {
        if byte & code as u8 != 0 {
            State::Click
        } else {
            State::Release
        }
    }
}

//todo add timestamp
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Button {
    code: Code,
    state: State,
    click_handlers: Vec<fn()>,
    doubleclick_handlers: Vec<fn()>,
    release_handlers: Vec<fn()>,
    updated: Duration,
    clicked: Duration,
}

impl Button {
    fn new(byte: u8, code: Code) -> Button {
        Button {
            code,
            state: State::get(byte, code),
            click_handlers: vec![],
            doubleclick_handlers: vec![],
            release_handlers: vec![],
            updated: Duration::new(0, 0),
            clicked: Duration::new(0, 0),
        }
    }
    pub fn on(&mut self, event: Event, f: fn()) {
        match event {
            Event::Click => self.click_handlers.push(f),
            Event::DoubleClick => self.doubleclick_handlers.push(f),
            Event::Release => self.release_handlers.push(f),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    x_difference: i8,
    y_difference: i8,
    left_button_state: State,
    right_button_state: State,
    middle_button_state: State,
    time: Duration,
}

impl Packet {
    pub fn new(x: i8, y: i8, buttons: u8, time: Duration) -> Packet {
        Packet {
            x_difference: x,
            y_difference: y,
            left_button_state: State::get(buttons, Code::Left),
            right_button_state: State::get(buttons, Code::Right),
            middle_button_state: State::get(buttons, Code::Middle),
            time,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Click,
    DoubleClick,
    Release,
}

#[derive(Debug, Clone)]
pub struct MouseInternal {
    x: u32,
    y: u32,
    left_button: Button,
    right_button: Button,
    middle_button: Button,
}

impl MouseInternal {
    fn new() -> MouseInternal {
        MouseInternal {
            x: 0,
            y: 0,
            left_button: Button::new(0, Code::Left),
            right_button: Button::new(0, Code::Right),
            middle_button: Button::new(0, Code::Middle),
        }
    }
    pub fn process_packet(&mut self, p: &Packet) {
        self.x = (self.x as i32 + p.x_difference as i32) as u32;
        self.y = (self.y as i32 + p.y_difference as i32) as u32;

        let is_doubleclicked = |b: &Button, new_state: State| -> bool {
            let in_time = p.time.sub(b.clicked).le(&Duration::from_secs_f32(0.5));
            in_time && new_state == State::Click
        };

        let dc = is_doubleclicked(&self.left_button, p.left_button_state);

        self.left_button.state = p.left_button_state;
        self.left_button.updated = p.time;
        if !dc && p.left_button_state == State::Click {
            self.left_button.clicked = duration_now();
            for click_handler in self.left_button.click_handlers.iter() {
                click_handler();
            }
        } else if dc {
            self.left_button.clicked = Duration::from_secs(0);
            for doubleclick_handler in self.left_button.doubleclick_handlers.iter() {
                doubleclick_handler();
            }
        }

        self.right_button.state = p.right_button_state;
        self.right_button.updated = p.time;

        let handlers = match p.right_button_state {
            State::Click => &self.right_button.click_handlers,
            State::Release => &self.right_button.release_handlers,
        };

        for handler in handlers.iter() {
            handler();
        }
    }
    pub fn left_button(&mut self) -> &mut Button {
        &mut self.left_button
    }
    pub fn right_button(&mut self) -> &mut Button {
        &mut self.right_button
    }
    pub fn middle_button(&mut self) -> &mut Button {
        &mut self.middle_button
    }
}
