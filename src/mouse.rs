use crate::println;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(u8)]
enum ButtonCode {
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
pub struct Button {
    code: ButtonCode,
    state: State,
    //timestamp:
}

impl Button {
    fn new(byte: u8, code: ButtonCode) -> Button {
        if byte & code as u8 == 1 {
            return Button {
                code,
                state: State::Click,
            };
        }
        Button {
            code,
            state: State::Release,
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
                Button::new(bytes[0], ButtonCode::Left),
                Button::new(bytes[0], ButtonCode::Right),
                Button::new(bytes[0], ButtonCode::Middle),
            ],
        }
    }
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
                Button::new(0, ButtonCode::Left),
                Button::new(0, ButtonCode::Right),
                Button::new(0, ButtonCode::Middle),
            ],
        }
    }
    pub fn process_packet(&mut self, p: &Packet) {
        self.x = (self.x as i32 + p.x_difference as i32) as u32;
        self.y = (self.y as i32 + p.y_difference as i32) as u32;
        //println!("x {} y {}", self.x, self.y);
    }
}
