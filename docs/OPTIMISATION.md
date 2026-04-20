# Performance Optimisation Notes

## The core tradeoff

STFT processing is inherently bursty. With FFT_SIZE=N and OVERLAP=4 (75% overlap):
- Hop = N/4 samples
- The FFT fires once every `hop / buffer_size` audio callbacks
- When it fires, it processes a full N-point FFT + IFFT + all bins at once

The audio deadline scales linearly with buffer size; the spike cost is fixed. So larger buffers
absorb spikes that would overrun a tight deadline:

| FFT Size | Hop     | Spike period @ 256/44.1k | Spike period @ 1024/96k |
|----------|---------|--------------------------|--------------------------|
| 2048     | 512     | every 2 callbacks        | every 0.5 callbacks      |
| 8192     | 2048    | every 8 callbacks        | every 2 callbacks        |
| 16384    | 4096    | every 16 callbacks       | every 4 callbacks        |

At 256 samples / 44.1 kHz the 16k FFT spike was ~3.5 ms against a 5.8 ms deadline (60%).
At 1024 samples / 96 kHz the same spike is ~1.875 ms against a 10.67 ms deadline (17%).

Users who want 16k resolution should use ≥ 512-sample buffers.

---

## Implemented

**SIMD runtime dispatch (Priority 1, partial)** — `apply_gains()` in
`spectral_compressor.rs` is annotated `#[multiversion(targets("x86_64+avx2+fma", "x86_64+sse4.1"))]`.
Pass 3 (gain application, `exp()`, wet/dry mix) auto-vectorises to AVX2/FMA on Haswell+ at
runtime; falls back to scalar on older hardware with no crash.  Passes 1 and 2 remain scalar
(Pass 1: per-bin `exp()` inside the envelope follower; Pass 2: prefix-sum has serial dependency).

---

## Priority 1 — SIMD vectorisation of the per-bin loop

**Where:** `src/dsp/engines/spectral_compressor.rs` — the inner loop over `0..num_bins` that
runs the envelope follower, gain computer, and smoothing pass.

**Gain:** AVX2 processes 8 × f32 per instruction. The loop is ~O(num_bins) scalar operations;
AVX2 reduces it to ~O(num_bins/8). For 16k that is 8193 bins → realistically 4–6× faster per
hop (some overhead from non-vectorisable branches).

**Compatibility:** AVX2 requires Haswell (Intel 2013+) / Zen 1 (AMD 2017+). Compiling with
`-C target-cpu=native` or `+avx2` produces a binary that crashes on older hardware.

**Solution — runtime dispatch:** the `multiversion` crate (`#[multiversion(...)]` proc macro)
compiles multiple versions of a function and selects the best one at runtime via CPUID. Users
without AVX2 fall back to SSE2 or scalar transparently, with no crash and only a speed
difference. This is the right approach for a distributed plugin.

```toml
# Cargo.toml
[dependencies]
multiversion = "0.8"
```

```rust
use multiversion::multiversion;

// The scalar fallback is generated automatically — do NOT include "default" in the list.
#[multiversion(targets("x86_64+avx2+fma", "x86_64+sse4.1"))]
fn process_bins_inner(/* ... */) { /* ... */ }
```

Alternatively, the `wide` crate provides portable SIMD types (`f32x8` etc.) that compile to
the best available ISA extension at build time — simpler but requires knowing the target at
compile time (less ideal for distribution).

---

## Priority 2 — Background thread (eliminates spikes entirely)

**Approach:** The audio thread becomes O(1): it copies input samples into a lock-free SPSC
ring buffer and copies processed output from another. A separate real-time-priority thread
does the FFT + per-bin work.

**Cost:** One extra hop of latency. At 16k / 44.1 kHz: 4096 samples = 93 ms additional
latency on top of the existing N/2 = 185 ms. Total ≈ 278 ms. Acceptable for mixing, not for
live monitoring.

**nih-plug hook:** `Plugin::BackgroundTask`. Wiring the STFT overlap-add loop to an async
task queue is a significant refactor — the `StftHelper` borrow model assumes the audio thread
owns the processing closure.

**Commercial precedent:** Soothe 2 and Gullfoss are believed to use this approach. It is the
only way to guarantee a flat audio-thread load profile regardless of FFT size.

---

## Priority 3 — Reduced-density per-bin processing

Analyse at full FFT resolution (good low-frequency detail) but compress in ~128 mel/bark
bands instead of per-bin. Interpolate band gains back to per-bin before applying.

- Per-hop CPU becomes nearly constant regardless of FFT size.
- Barely audible difference in practice; the band-to-bin interpolation is smooth.
- Closer to what commercial spectral processors are believed to do internally.

---

## Not worth it

**Variable overlap (OVERLAP=2):** Would halve average CPU and spike frequency. Blocked by
the fact that Hann² with 50% overlap does not satisfy COLA — reconstruction wobbles. Requires
designing a matched synthesis window. Not worth the complexity.

**GPU:** Not practical for a CLAP plugin running in a DAW host process.

**Larger SIMD (AVX-512):** Only Skylake-X and newer. Too narrow an install base.
