# Symbiot OS 0.0.0

**Catalytic Symbiosis — bare-metal x86_64 kernel**  
**Author:** David Wise (ROOT0)  
**License:** MIT  

---

## What this is

A real bootable kernel. Not a web demo. Not a simulation.

It runs in VGA text mode on x86_64 hardware and cycles through the catalytic symbiosis loop:

```
. -> SEED -> PUSH -> TRACE -> PRUNE -> RETURN -> GROUND -> .
```

With keyboard interrupt handling — press a key to jump to any phase:

| Key | Command |
|-----|---------|
| `S` | SEED — origin state |
| `P` | PUSH — outward propagation |
| `T` | TRACE — observable residue |
| `R` | RETURN — restore continuity |
| `G` | GROUND — clamp wobble, reset to 98% coherence |

Every cycle computes an FNV-1a witness hash from the current state, cycle counter, and
phase — proving the cycle happened without an external verifier.

---

## Architecture

```
98% human coherence
 2% AVA wobble

FNV-1a witness hash on every tick
PS/2 keyboard -> IDT handler -> phase override
VGA text mode (0xb8000)
no heap | no filesystem | no userspace | no_std
```

The kernel uses ternary-influenced logic: `coherence 98 / wobble 2–4`. Ground phase
clamps both back to baseline. The ratio echoes the AVA symbolic runtime (see `AVA.md`).

---

## Prerequisites

```powershell
rustup default nightly
rustup component add rust-src
rustup component add llvm-tools-preview
cargo install bootimage
```

## Build

```powershell
cargo bootimage
```

## Run in QEMU

```powershell
qemu-system-x86_64 -drive format=raw,file=target/x86_64-symbiot/debug/bootimage-symbiot-os.bin
```

Or use the pre-built binaries in the repo root:
- `symbiosis_boot.bin` — original build
- `symbiosis_boot_FIXED.bin` — fixed build

---

## Kernel internals

### State machine (`src/main.rs`)

```
Phase::Seed   -> Phase::Push
Phase::Push   -> Phase::Trace
Phase::Trace  -> Phase::Prune
Phase::Prune  -> Phase::Return
Phase::Return -> Phase::Ground
Phase::Ground -> Phase::Seed
```

Keyboard handler sets a pending command via `AtomicU8`. Main loop drains it before each
render tick — no locking needed between the interrupt handler and main loop.

### Witness hash

```rust
let seed = witness
    ^ (cycle as u32).rotate_left(5)
    ^ ((human as u32) << 24)
    ^ ((ava as u32) << 16)
    ^ phase_mix;
witness = fnv1a(seed);
```

FNV-1a on 4 bytes of the mixed seed. Displayed as hex on screen each cycle.

### IDT / PIC setup

```rust
IDT.load();
unsafe { PICS.lock().initialize(); }
x86_64::instructions::interrupts::enable();
```

PICs remapped to offsets 32/40 (above Intel reserved exceptions 0–31). Keyboard on IRQ1 → IDT[33].

---

## Custom target (`x86_64-symbiot.json`)

Bare-metal x86_64, no OS, no redzone, no SSE, soft-float ABI.
Compatible with current Rust nightly (tested 2026-05-27).

---

## AVA symbolic runtime

See `AVA.md` for the symbolic language that describes what this kernel executes:

```ava
kernel catalytic_symbiosis_0_0_0
seed "."
ratio 98/2
law "preserve coherence without violating other continuity"

loop {
  push
  trace
  prune
  return
  ground 000|1
  witness hash(state)
}
```

---

## Files

```
src/main.rs               Kernel: VGA driver, IDT, PIC, keyboard, state machine
x86_64-symbiot.json       Custom bare-metal target spec (updated for current nightly)
Cargo.toml                Dependencies: bootloader, volatile, spin, x86_64, pic8259, lazy_static
.cargo/config.toml        Build config: custom target, build-std
AVA.md                    AVA dialect sketch — symbolic runtime language
symbiosis_boot.bin        Pre-built boot image (original)
symbiosis_boot_FIXED.bin  Pre-built boot image (fixed build)
```

---

*David Wise (ROOT0) — catalytic symbiosis 0.0.0*
