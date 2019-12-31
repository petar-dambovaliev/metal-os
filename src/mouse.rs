use crate::println;
use crate::time::duration_now;
use alloc::{vec, vec::Vec};
use core::ops::Sub;
use core::sync::atomic::{AtomicU8, Ordering};
use core::time::Duration;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::port::Port;

lazy_static! {
    pub static ref MOUSE: Mutex<MouseInternal> = { Mutex::new(MouseInternal::new()) };
}

//Get Compaq Status Byte command
const GET_COMPAQ_STATUS_BYTE: u8 = 0x20;
//Set Compaq Status Byte command
const SET_COMPAQ_STATUS_BYTE: u8 = 0x60;
//Auxiliary Device command
const AUXILIARY_DEVICE_BYTE: u8 = 0xA8;
const IRQ12_BIT_POS: u8 = 1;
const MOUSE_CLOCK_BIT_POS: u8 = 5;
const WRITE: u8 = 0xD4;
const TIMEOUT: u32 = 100000;
const MOUSE_BIT: u8 = 0x01;
const DOUBLECLICK_TIMER: f32 = 0.5;

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
    fn is_doubleclicked(&self, new_state: State, dur: Duration, clicked_at: Duration) -> bool {
        dur.sub(clicked_at)
            .le(&Duration::from_secs_f32(DOUBLECLICK_TIMER))
            && new_state == State::Click
    }
    fn trigger_events(&mut self, new_state: State, dur: Duration) {
        self.state = new_state;
        self.updated = dur;

        let dc = self.is_doubleclicked(new_state, dur, self.clicked);
        let mut handlers: Vec<fn()> = vec![];

        if !dc && new_state == State::Click {
            self.clicked = duration_now();
            handlers.append(&mut self.click_handlers);
        } else if dc {
            self.clicked = Duration::from_secs(0);
            handlers.append(&mut self.doubleclick_handlers);
        } else if self.state == State::Release {
            handlers.append(&mut self.release_handlers);
        }

        for handler in handlers.iter() {
            handler();
        }
    }
}

#[derive(Debug, Clone)]
struct Packet {
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
    cmd_port: Port<u8>,
    data_port: Port<u8>,
    x: u32,
    y: u32,
    left_button: Button,
    right_button: Button,
    middle_button: Button,
}

impl MouseInternal {
    fn new() -> MouseInternal {
        MouseInternal {
            cmd_port: Port::new(0x64),
            data_port: Port::new(0x60),
            x: 0,
            y: 0,
            left_button: Button::new(0, Code::Left),
            right_button: Button::new(0, Code::Right),
            middle_button: Button::new(0, Code::Middle),
        }
    }
    fn process_packet(&mut self, p: &Packet) {
        self.x = (self.x as i32 + p.x_difference as i32) as u32;
        self.y = (self.y as i32 + p.y_difference as i32) as u32;

        self.left_button.trigger_events(p.left_button_state, p.time);
        self.right_button
            .trigger_events(p.right_button_state, p.time);
        self.middle_button
            .trigger_events(p.middle_button_state, p.time);
    }

    pub fn handler(&mut self) {
        lazy_static! {
            static ref INPUT: Mutex<[u8; 3]> = Mutex::new([0u8; 3]);
            static ref MOUSE_CYCLE: AtomicU8 = AtomicU8::new(0);
        }

        loop {
            let status = unsafe { self.cmd_port.read() };
            if !self.has_data(status) {
                break;
            }

            // the keyboard is on the same port
            // don't do anything if it's not a mouse packet
            if status & GET_COMPAQ_STATUS_BYTE == 0 {
                continue;
            }

            let data = unsafe { self.data_port.read() };
            let mut i = INPUT.lock();
            let ms = MOUSE_CYCLE.load(Ordering::Relaxed);

            match ms {
                0..=1 => {
                    i[ms as usize] = data;
                    MOUSE_CYCLE.fetch_add(1, Ordering::Relaxed);
                }
                2 => {
                    i[ms as usize] = data;
                    MOUSE_CYCLE.store(0, Ordering::Relaxed);
                    /* The top two bits of the first byte (values 0x80 and 0x40)
                    supposedly show Y and X overflows, respectively.
                    They are not useful.
                    If they are set, you should probably just discard the entire packet. */
                    if i[0] & 0x80 == 1 || i[0] & 0x40 == 1 {
                        println!("bad mouse packet {:?}", i);
                        continue;
                    }

                    let packet = Packet::new(i[0] as i8, i[1] as i8, i[2], duration_now());

                    self.process_packet(&packet);
                    let (x, y) = self.coordinates();
                    println!("x {} y {}", x, y);
                }
                _ => println!("unknown mouse cycle {}", ms),
            }
        }
    }

    pub fn init(&mut self) {
        unsafe {
            self.wait(true);
            self.cmd_port.write(AUXILIARY_DEVICE_BYTE);
            self.wait(true);
            self.cmd_port.write(GET_COMPAQ_STATUS_BYTE);
            self.wait(false);
        }

        let mut status_byte = unsafe { self.data_port.read() | 2 };
        self.wait(true);
        //enable the aux port to generate IRQ12
        status_byte |= 1 << IRQ12_BIT_POS;
        //Disable Mouse Clock
        status_byte &= !(1 << MOUSE_CLOCK_BIT_POS);

        unsafe {
            self.cmd_port.write(SET_COMPAQ_STATUS_BYTE);
            self.wait(true);
            self.data_port.write(status_byte);
            self.wait(true);
            let _ack = self.cmd_port.read();
        }

        self.write(0xF6);
        self.write(0xF4);
    }

    fn write(&mut self, b: u8) {
        self.wait(true);
        unsafe {
            self.cmd_port.write(WRITE);
        }
        self.wait(true);
        unsafe {
            self.data_port.write(b);
        }
    }

    fn wait(&mut self, b: bool) {
        for _ in 0..TIMEOUT {
            let status = unsafe { self.cmd_port.read() & MOUSE_BIT };
            if b == self.has_data(status) {
                return;
            }
        }
    }
    fn has_data(&self, b: u8) -> bool {
        return b & MOUSE_BIT != 0;
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
    pub fn coordinates(&self) -> (u32, u32) {
        (self.x, self.y)
    }
}
