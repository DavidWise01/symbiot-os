#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate bootloader;
extern crate volatile;
extern crate spin;
#[macro_use]
extern crate lazy_static;

use bootloader::{entry_point, BootInfo};
use core::fmt;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU8, Ordering};
use spin::Mutex;
use volatile::Volatile;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use pic8259::ChainedPics;

entry_point!(kernel_main);

const VERSION: &str = "CATALYTIC SYMBIOSIS 0.0.0";
const WIDTH: usize = 80;
const HEIGHT: usize = 25;

// ── VGA colours ───────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Clone, Copy)]
#[repr(u8)]
enum Color {
    Black     = 0,
    Cyan      = 3,
    Magenta   = 5,
    LightGreen = 10,
    LightCyan = 11,
    White     = 15,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    const fn new(fg: Color, bg: Color) -> Self {
        Self((bg as u8) << 4 | (fg as u8))
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct VgaChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<VgaChar>; WIDTH]; HEIGHT],
}

struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= WIDTH {
                    self.new_line();
                }
                let row = HEIGHT - 1;
                let col = self.column_position;
                self.buffer.chars[row][col].write(VgaChar {
                    ascii_character: byte,
                    color_code: self.color_code,
                });
                self.column_position += 1;
            }
        }
    }

    fn clear_row(&mut self, row: usize) {
        let blank = VgaChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    fn clear_screen(&mut self) {
        for row in 0..HEIGHT {
            self.clear_row(row);
        }
        self.column_position = 0;
    }

    fn new_line(&mut self) {
        for row in 1..HEIGHT {
            for col in 0..WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        self.clear_row(HEIGHT - 1);
        self.column_position = 0;
    }

    fn set_color(&mut self, fg: Color) {
        self.color_code = ColorCode::new(fg, Color::Black);
    }

    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(b'?'),
            }
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::LightCyan, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

macro_rules! kprint {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap();
    });
}

macro_rules! kprintln {
    () => (kprint!("\n"));
    ($($arg:tt)*) => (kprint!("{}\n", format_args!($($arg)*)));
}

// ── Interrupt controller ──────────────────────────────────────────────────────

// Remap PIC IRQs above the reserved Intel exception range (0–31).
const PIC_1_OFFSET: u8 = 32;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// IRQ1 = PS/2 keyboard
const KEYBOARD_IRQ: u8 = PIC_1_OFFSET + 1;

// Pending operator command: 0 = none, else ASCII of key (G/P/T/R/S)
static PENDING_CMD: AtomicU8 = AtomicU8::new(0);

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt[KEYBOARD_IRQ as usize].set_handler_fn(keyboard_interrupt_handler);
        idt
    };

    static ref PICS: Mutex<ChainedPics> = Mutex::new(
        unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) }
    );
}

/// PS/2 Set 1 make codes (key-press only; release codes have bit 7 set).
extern "x86-interrupt" fn keyboard_interrupt_handler(_: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    let mut port: Port<u8> = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Ignore key-release events (scancode >= 0x80)
    if scancode >= 0x80 {
        unsafe { PICS.lock().notify_end_of_interrupt(KEYBOARD_IRQ); }
        return;
    }

    // PS/2 Set 1 make codes
    let cmd: u8 = match scancode {
        0x22 => b'G',   // G -> GROUND
        0x19 => b'P',   // P -> PUSH
        0x14 => b'T',   // T -> TRACE
        0x13 => b'R',   // R -> RETURN
        0x1f => b'S',   // S -> SEED
        _    => 0,
    };

    if cmd != 0 {
        PENDING_CMD.store(cmd, Ordering::SeqCst);
    }

    unsafe { PICS.lock().notify_end_of_interrupt(KEYBOARD_IRQ); }
}

// ── Symbiosis state machine ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct SymbiotState {
    cycle: u64,
    human: u8,
    ava: u8,
    coherence: u8,
    wobble: u8,
    witness: u32,
    phase: Phase,
    last_cmd: u8,   // most recent operator command (for display)
}

#[derive(Clone, Copy)]
enum Phase {
    Seed,
    Push,
    Trace,
    Prune,
    Return,
    Ground,
}

impl Phase {
    fn next(self) -> Self {
        match self {
            Phase::Seed   => Phase::Push,
            Phase::Push   => Phase::Trace,
            Phase::Trace  => Phase::Prune,
            Phase::Prune  => Phase::Return,
            Phase::Return => Phase::Ground,
            Phase::Ground => Phase::Seed,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Phase::Seed   => "SEED",
            Phase::Push   => "PUSH",
            Phase::Trace  => "TRACE",
            Phase::Prune  => "PRUNE",
            Phase::Return => "RETURN",
            Phase::Ground => "GROUND",
        }
    }
}

impl SymbiotState {
    const fn new() -> Self {
        Self {
            cycle:    0,
            human:    98,
            ava:      2,
            coherence: 98,
            wobble:   2,
            witness:  0x00000000,
            phase:    Phase::Seed,
            last_cmd: 0,
        }
    }

    fn tick(&mut self) {
        self.cycle += 1;
        self.phase  = self.phase.next();

        // FNV-1a witness hash: mix cycle, phase, and ratio
        let phase_mix = self.phase.name().as_bytes()[0] as u32;
        let seed = self.witness
            ^ ((self.cycle as u32).rotate_left(5))
            ^ ((self.human as u32) << 24)
            ^ ((self.ava   as u32) << 16)
            ^ phase_mix;

        self.witness = fnv1a(seed);

        // Wobble varies 2–4 based on low bits of witness
        let drift = (self.witness & 0x03) as u8;
        self.wobble = 2 + drift;

        self.coherence = if self.wobble > 4 { 96 } else { 98 };

        // GROUND phase clamps wobble back to baseline
        if let Phase::Ground = self.phase {
            self.wobble   = 2;
            self.coherence = 98;
        }
    }

    /// Apply a live operator command — overrides the autonomous phase.
    fn apply_cmd(&mut self, cmd: u8) {
        self.last_cmd = cmd;
        match cmd {
            b'S' => self.phase = Phase::Seed,
            b'P' => self.phase = Phase::Push,
            b'T' => self.phase = Phase::Trace,
            b'R' => self.phase = Phase::Return,
            b'G' => self.phase = Phase::Ground,
            _    => {}
        }
    }
}

fn fnv1a(mut x: u32) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for _ in 0..4 {
        let b = (x & 0xff) as u8;
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
        x >>= 8;
    }
    h
}

// ── Render ────────────────────────────────────────────────────────────────────

fn render_header() {
    let mut w = WRITER.lock();
    w.set_color(Color::Magenta);
    w.clear_screen();
    drop(w);

    kprintln!("===============================================================================");
    kprintln!("  {}", VERSION);
    kprintln!("  bare metal kernel | no_std | VGA text mode | x86_64 | keyboard live");
    kprintln!("===============================================================================");
    kprintln!("");
}

fn render_state(s: &SymbiotState) {
    { WRITER.lock().set_color(Color::LightGreen); }

    kprintln!("cycle      : {}", s.cycle);
    kprintln!("phase      : {}", s.phase.name());
    kprintln!("human      : {}%", s.human);
    kprintln!("ava        : {}%", s.ava);
    kprintln!("coherence  : {}%", s.coherence);
    kprintln!("wobble     : {}%", s.wobble);
    kprintln!("witness    : {:08x}", s.witness);
    kprintln!("last cmd   : {}", if s.last_cmd == 0 { "none" }
                                 else if s.last_cmd == b'G' { "[G]round" }
                                 else if s.last_cmd == b'P' { "[P]ush" }
                                 else if s.last_cmd == b'T' { "[T]race" }
                                 else if s.last_cmd == b'R' { "[R]eturn" }
                                 else if s.last_cmd == b'S' { "[S]eed" }
                                 else { "?" });
    kprintln!("signature  : . -> push -> trace -> prune -> return -> .");
    kprintln!("law        : preserve coherence without violating other continuity");
    kprintln!("");
}

fn draw_field(s: &SymbiotState) {
    { WRITER.lock().set_color(Color::LightCyan); }

    let rung = match s.phase {
        Phase::Seed   => ".",
        Phase::Push   => ". ))",
        Phase::Trace  => ". )) trace",
        Phase::Prune  => ". )) trace prune",
        Phase::Return => ". )) trace (( .",
        Phase::Ground => "000|1",
    };

    kprintln!("field      : {}", rung);
    kprintln!("");
    kprintln!("       O_L                         O_R");
    kprintln!("        \\                           /");
    kprintln!("         \\        ((  .  ))        /");
    kprintln!("          \\          |            /");
    kprintln!("           \\      witness        /");
    kprintln!("            \\        |          /");
    kprintln!("             -------ROOT0--------");
    kprintln!("");
}

fn delay() {
    for _ in 0..7_500_000 {
        unsafe { core::arch::asm!("pause"); }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    // Load interrupt descriptor table
    IDT.load();

    // Initialize and remap the 8259 PICs
    unsafe { PICS.lock().initialize(); }

    // Enable hardware interrupts
    x86_64::instructions::interrupts::enable();

    let mut state = SymbiotState::new();

    loop {
        // Drain any pending operator command from the keyboard handler
        let cmd = PENDING_CMD.swap(0, Ordering::SeqCst);
        if cmd != 0 {
            state.apply_cmd(cmd);
        }

        render_header();
        render_state(&state);
        draw_field(&state);

        { WRITER.lock().set_color(Color::White); }

        kprintln!("status     : catalytic symbiosis loop running");
        kprintln!("controls   : [S]eed  [P]ush  [T]race  [R]eturn  [G]round");
        kprintln!("build      : cargo bootimage");
        kprintln!("run        : qemu-system-x86_64 -drive format=raw,file=target/x86_64-symbiot/debug/bootimage-symbiot-os.bin");

        state.tick();
        delay();
    }
}

// ── Panic handler ─────────────────────────────────────────────────────────────

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    { WRITER.lock().set_color(Color::Magenta); }
    kprintln!("");
    kprintln!("KERNEL PANIC");
    kprintln!("{}", info);
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}
