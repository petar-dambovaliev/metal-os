use super::mouse;
use crate::gdt;
use crate::hlt_loop;
use crate::print;
use crate::println;
use core::borrow::Borrow;
use core::sync::atomic::{AtomicU8, Ordering};
use lazy_static::lazy_static;
use pic8259_simple::ChainedPics;
use spin;
use spin::Mutex;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

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

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        mouse_init();

        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[SecondaryInterruptIndex::Mouse.as_usize()].set_handler_fn(mouse_interrupt_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt
    };
}

pub fn mouse_init() {
    let mut cmd_port: Port<u8> = Port::new(0x64);
    let mut data_port: Port<u8> = Port::new(0x60);

    unsafe {
        mouse_wait(true);
        cmd_port.write(AUXILIARY_DEVICE_BYTE);
        mouse_wait(true);
        cmd_port.write(GET_COMPAQ_STATUS_BYTE);
        mouse_wait(false);
    }

    let mut status_byte = unsafe { data_port.read() | 2 };
    mouse_wait(true);
    //enable the aux port to generate IRQ12
    status_byte |= 1 << IRQ12_BIT_POS;
    //Disable Mouse Clock
    status_byte &= !(1 << MOUSE_CLOCK_BIT_POS);

    unsafe {
        cmd_port.write(SET_COMPAQ_STATUS_BYTE);
        mouse_wait(true);
        data_port.write(status_byte);
        mouse_wait(true);
        let ack = cmd_port.read();
    }

    mouse_write(0xF6);
    mouse_write(0xF4);
}

fn mouse_wait(b: bool) {
    let mut cmd_port: Port<u8> = Port::new(0x64);
    for _ in 0..TIMEOUT {
        let status = unsafe { cmd_port.read() & MOUSE_BIT };
        if b == is_mouse_bit(status) {
            return;
        }
    }
}

fn mouse_write(b: u8) {
    let mut cmd_port: Port<u8> = Port::new(0x64);
    let mut data_port: Port<u8> = Port::new(0x60);
    mouse_wait(true);
    unsafe {
        cmd_port.write(WRITE);
    }
    mouse_wait(true);
    unsafe {
        data_port.write(b);
    }
}

fn is_mouse_bit(b: u8) -> bool {
    return b & MOUSE_BIT == 1;
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    let mut cmd_port: Port<u8> = Port::new(0x64);
    let mut data_port: Port<u8> = Port::new(0x60);

    lazy_static! {
        static ref INPUT: Mutex<[u8; 3]> = Mutex::new([0u8; 3]);
        static ref MOUSE_CYCLE: AtomicU8 = AtomicU8::new(0);
        static ref MOUSE: mouse::Mouse = mouse::Mouse::new();
    }

    while let status = unsafe { cmd_port.read() } {
        if !is_mouse_bit(status) {
            break;
        }
        let data = unsafe { data_port.read() };
        if data & GET_COMPAQ_STATUS_BYTE == 0 {
            continue;
        }

        let mut i = INPUT.lock();

        let ms = MOUSE_CYCLE.load(Ordering::Relaxed);
        unsafe {
            match ms {
                0..=1 => {
                    i[ms as usize] = data;
                    MOUSE_CYCLE.fetch_add(1, Ordering::Relaxed);
                }
                2 => {
                    MOUSE_CYCLE.store(0, Ordering::Relaxed);
                    /* The top two bits of the first byte (values 0x80 and 0x40)
                    supposedly show Y and X overflows, respectively.
                    They are not useful.
                    If they are set, you should probably just discard the entire packet. */
                    if i[0] & 0x80 == 1 || i[0] & 0x40 == 1 {
                        panic!("bad packet");
                    }

                    let packet = mouse::Packet::from(i.deref());
                    MOUSE.process_packet(&packet);
                }
                _ => {}
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(SecondaryInterruptIndex::Mouse.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, Keyboard, ScancodeSet1};

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => print!("{}", character),
                DecodedKey::RawKey(key) => print!("{:?}", key),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum SecondaryInterruptIndex {
    RTClock = PIC_2_OFFSET,
    ACPI,
    Available1,
    Available2,
    Mouse,
}

impl SecondaryInterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: &mut InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: &mut InterruptStackFrame) {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

#[cfg(test)]
use crate::{serial_print, serial_println};
use core::ops::Deref;

#[test_case]
fn test_breakpoint_exception() {
    serial_print!("test_breakpoint_exception...");
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
    serial_println!("[ok]");
}
