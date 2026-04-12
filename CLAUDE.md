# Spectral Balance — AI Assistant Guide

This document is for AI assistants (Claude, etc.) working on this codebase.

## What it is

A Soothe-style **spectral compressor** CLAP plugin for Linux/Bitwig, written in Rust.
It performs per-FFT-bin dynamic gain reduction controlled by 7 drawn parameter curves.

Target: Linux only. Primary host: Bitwig Studio.
Patent-safe design — does not use the oeksound-patented Hilbert/convolution approach.

## Build

```bash
# Debug (fast compile, slow)
cargo build

# Release (optimised, what you want to profile)
cargo build --release

# Bundle as .clap file (installs go to target/bundled/)
cargo run --package xtask -- bundle spectral_forge --release

# Install to Bitwig's search path
cp target/bundled/spectral_forge.clap ~/.clap/
```

## Test

```bash
cargo test          # 14 tests across 4 test files
cargo test engine   # engine_contract tests only
cargo test stft     # stft_roundtrip only
```

Test files live in `tests/`. They use the library crate (`rlib` target) — the `crate-type` in Cargo.toml includes both `cdylib` (the plugin) and `rlib` (for tests).

## Architecture overview

```
src/
  lib.rs              — Plugin entry point: Plugin/ClapPlugin impl, initialize/reset/process
  params.rs           — All nih-plug Params: floats, bools, enums, persisted curve_nodes
  bridge.rs           — SharedState: triple-buffer channels GUI↔Audio, AtomicF32
  editor_ui.rs        — create_editor(): builds the egui GUI
  editor/
    curve.rs          — CurveNode, compute_curve_response(), curve_widget(), paint_response_curve()
    spectrum_display.rs — paint_spectrum(): log-freq bars from FFT magnitudes
    suppression_display.rs — paint_suppression(): stalactite bars (gain reduction per bin)
    theme.rs          — ALL visual constants (colours, sizes). Edit only here.
    mod.rs            — pub use
  dsp/
    pipeline.rs       — Pipeline: STFT overlap-add, M/S encode, sidechain STFT, BinParams assembly
    engines/
      mod.rs          — SpectralEngine trait + BinParams<'_> struct + EngineSelection enum
      spectral_compressor.rs — SpectralCompressorEngine: envelope→gain_computer→smooth→apply
    guard.rs          — flush_denormals(), sanitize() (clamp NaN/Inf before FFT)
```

## Key constants

```rust
FFT_SIZE = 2048
NUM_BINS = FFT_SIZE / 2 + 1  // 1025
OVERLAP  = 4                  // 75% overlap, hop = 512 samples
NORM     = 2.0 / (3.0 * FFT_SIZE as f32)  // Hann² OLA normalisation
NUM_CURVE_SETS = 7            // threshold, ratio, attack, release, knee, makeup, mix
NUM_NODES      = 6            // nodes per curve (indices 0,5 = shelves; 1-4 = bells)
```

## The 7 curve channels

Each curve is a `[CurveNode; 6]` → `Vec<f32>` of per-bin gains via `compute_curve_response()`.
The pipeline maps linear gains (1.0 = neutral) to physical units:

| Index | Curve      | 1.0 maps to          | Range         |
|-------|------------|----------------------|---------------|
| 0     | THRESHOLD  | -20 dBFS             | -60 … 0 dBFS  |
| 1     | RATIO      | 1:1 (no compression) | 1:1 … 20:1    |
| 2     | ATTACK     | global attack × 1    | 0.1 … 500 ms  |
| 3     | RELEASE    | global release × 1   | 1 … 2000 ms   |
| 4     | KNEE       | 6 dB soft knee       | 0 … 24 dB     |
| 5     | MAKEUP     | 0 dB makeup          | log-scaled     |
| 6     | MIX        | 100% wet             | 0 … 100%      |

Bridge defaults are all `1.0` — the neutral value for every curve. `initialize()` pushes curves
computed from persisted `curve_nodes` on startup so restored sessions load correctly.

## Data flow

```
GUI curve editor → compute_curve_response() → triple_buffer::Input → [audio thread]
                                                                         ↓
                                            Pipeline::process() → copy_from_slice(curve_rx.read())
                                                                 → map to BinParams
                                                                 → STFT → SpectralEngine
                                                                 → spectrum/suppression triple_buffer
                                                                         ↓
                                                              GUI spectrum/suppression display
```

## Real-time safety rules (NEVER break these)

- **No allocation on the audio thread.** `Vec::clone()`, `Vec::new()`, `collect()` are all forbidden inside `Pipeline::process()` and `SpectralEngine::process_bins()`. Use pre-allocated buffers.
- **No locking on the audio thread.** Use `try_lock()` only in the GUI thread. The audio thread uses triple-buffer (`curve_rx[i].read()` is lock-free).
- **No I/O on the audio thread.** No file access, no `println!`.
- `assert_process_allocs` feature is enabled in Cargo.toml — it will abort if the audio thread allocates.
- The `guard::flush_denormals()` call at the top of `process()` sets FTZ+DAZ CPU flags each block to prevent denormal slowdowns.

## CurveNode coordinate system

```
x: 0.0 = 20 Hz, 1.0 = 20 kHz  (log-linear: freq = 20 * 1000^x)
y: -1.0 = -18 dB, 0.0 = neutral, +1.0 = +18 dB
q: 0.0 = 4 octaves bandwidth, 1.0 = 0.1 octave bandwidth  (0.1 * 40^q octaves)
```

Nodes 0 and 5 are shelves (low/high). Nodes 1–4 are Gaussian bells.
`compute_curve_response()` returns a `Vec<f32>` of linear multipliers, one per FFT bin.

## BinParams<'_>

All slices are `num_bins` long. `process_bins()` must not allocate and must fill `suppression_out`
completely with non-negative finite values (NaN sentinel test in `engine_contract.rs`).

## Triple-buffer protocol

```rust
// GUI → Audio (write side, from GUI thread):
let mut tx = curve_tx[i].try_lock().unwrap();
tx.input_buffer_mut().copy_from_slice(&gains);
tx.publish();

// Audio → GUI (write side, from audio thread — already mutable, no lock needed):
shared.spectrum_tx.input_buffer_mut().copy_from_slice(&spectrum_buf);
shared.spectrum_tx.publish();

// GUI read side (try_lock to avoid blocking):
if let Some(mut rx) = spectrum_rx.try_lock() {
    paint_spectrum(painter, rect, rx.read());
}
```

## Stereo modes (`params.stereo_link`)

- **Linked** (default): single engine processes both channels identically.
- **Independent**: `engine` handles ch0, `engine_r` handles ch1. Both are separate SpectralCompressorEngine instances reset at the same time.
- **MidSide**: L/R → M/S (FRAC_1_SQRT_2 matrix) before STFT, decode after. Single engine.

## Adding a new SpectralEngine

1. Add a variant to `EngineSelection` in `engines/mod.rs`.
2. Add a new module in `engines/`, implement `SpectralEngine` trait.
3. Wire the variant in `create_engine()`.
4. The engine receives `BinParams<'_>` — read the field doc-comments before using them.
5. Write at least one test in `tests/engine_contract.rs` covering the new variant.
6. Override `tail_length()` if the engine has a longer tail than one FFT window (e.g. Freeze mode).

## Gotchas

- `StftHelper::process_overlap_add()` takes `&mut self` by borrow inside the closure, so all other `self.*` fields must be rebound as locals before the call to avoid conflicting borrows.
- `triple_buffer::Output::read()` takes `&mut self` — each read() must be a separate statement.
- `curve_cache` in `Pipeline` is `[Vec<f32>; 7]` — populated by `copy_from_slice` each block, referenced by index in the parameter mapping loop. Rust 2021 split-field borrows allow reading `curve_cache[i]` while writing `bp_threshold[k]` etc. simultaneously.
- All visual constants live in `editor/theme.rs` — do not hardcode colours or sizes elsewhere.
- The sidechain STFT (`sc_stft`) uses a separate `StftHelper` with 2 channels regardless of plugin layout.
