use crate::println;
use x86_64::instructions::port::Port;

pub static mut CHAN0: Port<u8> = Port::new(0x40);
pub static mut CHAN1: Port<u8> = Port::new(0x41);
pub static mut CHAN2: Port<u8> = Port::new(0x42);
pub static mut COMMAND: Port<u8> = Port::new(0x43);

static SELECT_CHAN0: u8 = 0;
static LOHI: u8 = 0x30;

static CHAN0_DIVISOR: u16 = 2685;

pub fn init() {
    unsafe {
        COMMAND.write(SELECT_CHAN0 | LOHI | 5);
        CHAN0.write((CHAN0_DIVISOR & 0xFF) as u8);
        CHAN0.write((CHAN0_DIVISOR >> 8) as u8);
    }
    println!("Using PIT");
}
