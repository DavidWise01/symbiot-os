# Symbiot OS 0.0.0

[![build](https://github.com/DavidWise01/symbiot-os/actions/workflows/build.yml/badge.svg)](https://github.com/DavidWise01/symbiot-os/actions/workflows/build.yml)

**Catalytic Symbiosis — bare-metal x86_64 kernel**  
**Author:** David Wise (ROOT0)  
**License:** MIT  

> CI builds the kernel and a bootable image on every push (nightly Rust, `build-std`,
> custom JSON target) and uploads `bootimage-symbiot-os.bin` as a downloadable artifact.

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

## What you'll see

The screen the kernel paints, reconstructed from the VGA draw code in `src/main.rs`
(`render_header` → `render_state` → `draw_field`). The header prints magenta, the state
block light-green, the field light-cyan, on a black 80×25 text console:

```
===============================================================================
  CATALYTIC SYMBIOSIS 0.0.0
  bare metal kernel | no_std | VGA text mode | x86_64 | keyboard live
===============================================================================

cycle      : 42
phase      : TRACE
human      : 98%
ava        : 2%
coherence  : 98%
wobble     : 3%
witness    : 6f3a91c4
last cmd   : [T]race
signature  : . -> push -> trace -> prune -> return -> .
law        : preserve coherence without violating other continuity

field      : . )) trace

       O_L                         O_R
        \                           /
         \        ((  .  ))        /
          \          |            /
           \      witness        /
            \        |          /
             -------ROOT0--------
```

`cycle`, `witness`, and `phase` advance every tick; pressing `S/P/T/R/G` overrides the
phase live via the keyboard interrupt, and `field` redraws to match. Values above are a
representative mid-run frame — not a hardware capture.

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

Recent nightlies gate JSON target-spec files behind a flag, so `.cargo/config.toml`
sets `json-target-spec = true` under `[unstable]`. Without it the build fails with
`` `.json` target specs require -Zjson-target-spec ``. Verified building (kernel +
`cargo bootimage`) on nightly 2026-05-15; CI re-verifies on every push.

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
x86_64-symbiot.json       Custom bare-metal target spec
Cargo.toml                Dependencies: bootloader, volatile, spin, x86_64, pic8259, lazy_static
.cargo/config.toml        Build config: custom target, build-std, json-target-spec
rust-toolchain.toml       Pins nightly + rust-src + llvm-tools-preview
.github/workflows/build.yml  CI: builds kernel + boot image, uploads artifact
AVA.md                    AVA dialect sketch — symbolic runtime language
symbiosis_boot.bin        Pre-built boot image (original)
symbiosis_boot_FIXED.bin  Pre-built boot image (fixed build)
```

---

*David Wise (ROOT0) — catalytic symbiosis 0.0.0*
