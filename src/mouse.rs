use crate::println;

const LEFT: u8 = 0x01;
const RIGHT: u8 = 0x02;
const MIDDLE: u8 = 0x04;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Click {
    click: u8,
}

impl From<u8> for Click {
    fn from(b: u8) -> Self {
        let mut click: u8 = 0;

        if b & LEFT == 1 {
            click |= LEFT
        }
        if b & RIGHT == 1 {
            click |= RIGHT
        }
        if b & MIDDLE == 1 {
            click |= MIDDLE
        }
        Click { click }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Packet {
    magic: u32,
    x_difference: i8,
    y_difference: i8,
    click: Click,
}

const MOUSE_MAGIC: u32 = 0xFEED1234;

impl From<&[u8; 3]> for Packet {
    fn from(bytes: &[u8; 3]) -> Self {
        Self {
            magic: MOUSE_MAGIC,
            x_difference: bytes[1] as i8,
            y_difference: bytes[2] as i8,
            click: Click::from(bytes[0]),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Mouse {}

impl Mouse {
    pub fn new() -> Mouse {
        Mouse {}
    }
    pub fn process_packet(self, p: &Packet) {
        println!("packet: {:?}", p)
    }
}
