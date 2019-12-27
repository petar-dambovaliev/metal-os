use crate::println;
use alloc::{vec, vec::Vec};

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

//todo add timestamp
#[derive(Debug, PartialEq, Eq, Clone)]
struct Button {
    code: Code,
    state: State,
    click_handlers: Vec<fn()>,
    doubleclick_handlers: Vec<fn()>,
    release_handlers: Vec<fn()>,
    //timestamp:
}

impl Button {
    fn new(byte: u8, code: Code) -> Button {
        let state = if byte & code as u8 == 1 {
            State::Click
        } else {
            State::Release
        };

        Button {
            code,
            state,
            click_handlers: vec![],
            doubleclick_handlers: vec![],
            release_handlers: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    x_difference: i8,
    y_difference: i8,
    buttons: [Button; 3],
}

impl From<&[u8; 3]> for Packet {
    fn from(bytes: &[u8; 3]) -> Packet {
        Packet {
            x_difference: bytes[1] as i8,
            y_difference: bytes[2] as i8,
            buttons: [
                Button::new(bytes[0], Code::Left),
                Button::new(bytes[0], Code::Right),
                Button::new(bytes[0], Code::Middle),
            ],
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
pub struct Mouse {
    x: u32,
    y: u32,
    buttons: [Button; 3],
}

impl Mouse {
    pub fn new() -> Mouse {
        Mouse {
            x: 0,
            y: 0,
            buttons: [
                Button::new(0, Code::Left),
                Button::new(0, Code::Right),
                Button::new(0, Code::Middle),
            ],
        }
    }
    pub fn process_packet(&mut self, p: &Packet) {
        self.x = (self.x as i32 + p.x_difference as i32) as u32;
        self.y = (self.y as i32 + p.y_difference as i32) as u32;
        //println!("x {} y {}", self.x, self.y);
    }
    pub fn on(&mut self, code: Code, event: Event, f: fn()) {
        for button in self.buttons.iter_mut() {
            if button.code == code {
                match event {
                    Event::Click => button.click_handlers.push(f),
                    Event::DoubleClick => button.doubleclick_handlers.push(f),
                    Event::Release => button.release_handlers.push(f),
                }
                return;
            }
        }
    }
}
