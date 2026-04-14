# Spectral Forge — Technical Reference

This document describes all processing features in detail, including the mathematical formulas used throughout the signal chain.

---

## Table of contents

1. [Signal flow overview](#1-signal-flow-overview)
2. [STFT engine](#2-stft-engine)
3. [Parameter curves](#3-parameter-curves)
4. [BinParams assembly](#4-binparams-assembly)
5. [Spectral compressor engine](#5-spectral-compressor-engine)
6. [Effects pass](#6-effects-pass)
7. [Spectral contrast engine](#7-spectral-contrast-engine)
8. [Sidechain processing](#8-sidechain-processing)
9. [Stereo modes](#9-stereo-modes)
10. [GUI parameter reference](#10-gui-parameter-reference)

---

## 1. Signal flow overview

```
Audio in → input gain → [M/S encode if MidSide] → STFT analysis
                                                         ↓
                                              per-hop STFT frame (complex bins)
                                                         ↓
                                              ┌─ Spectral compressor ─┐
                                              │  envelope → GR → apply│
                                              └───────────────────────┘
                                                         ↓
                                              ┌─ Effects pass ─────────┐
                                              │  Freeze / PhaseRand /  │
                                              │  SpectralContrast       │
                                              └────────────────────────┘
                                                         ↓
                                              STFT synthesis (overlap-add)
                                                         ↓
                                    [M/S decode if MidSide] → output gain → Audio out
```

The compressor and effects pass run in series on the same STFT frames. Both are always active; running the contrast enhancer after the compressor is the intended use case (compress resonant peaks, then expand natural spectral variation).

---

## 2. STFT engine

**Parameters:**

| Symbol | Value | Description |
|--------|-------|-------------|
| N | 2048 | FFT size |
| K | N/2 + 1 = 1025 | Number of complex bins (one-sided) |
| R | N/4 = 512 | Hop size (75% overlap) |
| W(n) | Hann | Analysis and synthesis window |
| norm | 2 / (3N) | OLA normalisation for Hann² at 75% overlap |

The plugin uses `rubato`-backed `StftHelper` from `nih_plug`. Each hop:

1. Apply Hann window to the latest R new samples.
2. Forward FFT (real-to-complex via `realfft`): produces K complex bins.
3. Call `engine.process_bins()` on the bins.
4. Call effects pass on the same bins.
5. Inverse FFT (complex-to-real).
6. Apply Hann synthesis window and overlap-add into the output buffer.

**DC and Nyquist bins** (k=0 and k=K−1) must remain real-valued (imaginary part = 0) at all times to satisfy the Hermitian symmetry required by `realfft`. Any effect that modifies phases must skip these two bins.

---

## 3. Parameter curves

Seven per-frequency curves shape the compressor behaviour. Each curve is a set of six nodes:

```
index 0: Low shelf   (left edge of frequency range)
index 1–4: Bell bands
index 5: High shelf  (right edge)
```

### Node coordinate system

```
x ∈ [0, 1]   normalised log-frequency: f = 20 × 1000^x  (Hz)
              x=0 → 20 Hz, x=0.5 → 632 Hz, x=1 → 20 kHz
y ∈ [−1, +1]  gain offset: y=0 is neutral
              physical gain: 0 dB + y × 18 dB
q ∈ [0, 1]   normalised bandwidth: bw = 0.1 × 40^q octaves
              q=0 → 4 oct (very wide bell), q=1 → 0.1 oct (very narrow)
```

### Bell filter magnitude

```
magnitude(f) = 1 + (10^(G/20) − 1) × exp(−(log₂(f/f₀))² / (2σ²))

where σ = bw / 2.355   (octave bandwidth to Gaussian sigma)
      G = y × 18  dB
      f₀ = 20 × 1000^x  Hz
```

### Shelf filter magnitude

```
t = clamp((±log₂(f/f₀) + tw/2) / tw, 0, 1)   (+ for high shelf, − for low shelf)
tw = 2 + bw                                     (transition width in octaves)
s = smoothstep(t) = 3t² − 2t³                   (S-curve 0→1)
magnitude(f) = 1 + (G_linear − 1) × s
```

### Combined response

All six bands multiply:

```
curve_response[k] = ∏ᵢ magnitude_i(f_k)
```

where `f_k = k × sample_rate / N` Hz.

The result is a vector of linear multipliers — 1.0 is neutral for every curve.

---

## 4. BinParams assembly

The pipeline maps each curve's linear gain vector to physical units before passing them to the engine.

| Curve | Neutral (gain=1) | Formula | Range |
|-------|-----------------|---------|-------|
| THRESHOLD | −20 dBFS | `−20 + 20·log₁₀(g) × (60/18)` | −80…0 dBFS |
| RATIO | 1:1 | `g` (clipped ≥1 for compressor, 0–20 for contrast) | 1…20 |
| ATTACK | global Atk × 1 | `Atk_global × freq_scale(k) × g` | 0.1…500 ms |
| RELEASE | global Rel × 1 | `Rel_global × freq_scale(k) × g` | 1…2000 ms |
| KNEE | 6 dB | `g × 6 dB` | 0…48 dB |
| MAKEUP | 0 dB | `20·log₁₀(g)` | ±36 dB |
| MIX | 100% wet | `g` | 0…1 |

**Threshold modifiers** applied after curve mapping:

```
threshold[k] = (−20 + t_db × (60/18) + slope × log₂(f_k/1000) + offset) clipped to [−80, 0]
```

where `slope` is the Tilt parameter (dB/octave) and `offset` is the Th Off parameter (dB).

**Frequency-dependent time scaling:**

```
freq_scale(k) = (1000 / f_k)^(Freq × 0.5)

attack[k]  = Atk_global × freq_scale(k) × curve_attack[k]
release[k] = Rel_global × freq_scale(k) × curve_release[k]
```

High-frequency bins get shorter times, low-frequency bins get longer times, when Freq > 0.

**Sensitivity** modifies the effective threshold to make the compressor relative rather than absolute:

```
effective_threshold[k] = threshold[k] × (1 − sensitivity) + envelope[k] × sensitivity
```

where `envelope[k]` is the current smoothed magnitude at bin k, converted to dBFS.

---

## 5. Spectral compressor engine

`SpectralCompressorEngine` implements per-bin dynamic range compression using a feed-forward architecture with a gain computer and envelope follower.

### Gain computer (static curve)

Given input magnitude `x_db` (dBFS) and threshold `T`, ratio `R`, knee width `W`:

```
above = x_db − T

Hard knee (W < 0.001):
  GR = max(0, above × (1 − 1/R))

Soft knee (W ≥ 0.001):
  if above < −W/2:
    GR = 0
  elif above > W/2:
    GR = above × (1 − 1/R)
  else:
    GR = (above + W/2)² / (2W) × (1 − 1/R)   [quadratic knee]
```

`GR` is always ≥ 0 (gain reduction, a positive magnitude to subtract from dBFS level).

### Envelope follower

One-pole IIR smoother per bin, with separate attack and release coefficients:

```
coeff = exp(−1 / time_hops)    where time_hops = time_ms × sample_rate / hop_size

If GR_new > GR_env[k]:   coeff = coeff_attack    (attack: catching up to increase)
Else:                      coeff = coeff_release   (release: recovering from decrease)

GR_env[k] = coeff × GR_env[k] + (1 − coeff) × GR_new
```

### Gain mask smoothing

The raw per-bin GR values are blurred in log-frequency using a prefix-sum box filter:

```
width_ratio = 2^(smoothing_semitones / 12)
k_lo = floor(k / width_ratio)
k_hi = ceil(k × width_ratio)

GR_smooth[k] = mean(GR_env[k_lo..=k_hi])
```

This prevents abrupt band-to-band gain differences from creating audible artifacts. Implemented in O(N) via prefix sums on the GR array.

### Auto makeup

When auto makeup is enabled, a long-term average (τ ≈ 1000 ms) of the smoothed GR is tracked per bin. This is subtracted from the total applied gain to compensate for the average loudness reduction:

```
auto_makeup[k] += (1 − coeff_slow) × (GR_smooth[k] × mix[k] − auto_makeup[k])
total_gain_db = −GR_smooth[k] + makeup_db[k] + auto_makeup[k]
```

### Application

```
linear_gain = 10^(total_gain_db / 20)
bins[k] = bins[k] × ((1 − mix[k]) + mix[k] × linear_gain)
```

Dry/wet mix is applied per bin using the MIX curve value.

### Delta monitor

When the delta monitor is active, the output is `input − compressed_output` — i.e., only what is being removed is passed through. Implemented by inverting the gain:

```
delta_gain = (1 − mix) × linear_gain − mix
bins[k] = bins[k] × delta_gain
```

---

## 6. Effects pass

The effects pass runs after the compressor on the same complex bins. The selected mode determines which processing (if any) is applied.

### FREEZE

Captures a complete complex STFT frame (all K complex numbers, including both magnitude and phase) and replaces subsequent frames with the captured frame:

```
on freeze:   frozen_bins = complex_buf[0..K]
each hop:    complex_buf = frozen_bins
```

Capturing full complex values (not just magnitudes) is essential — holding only magnitudes while allowing the phase to continue evolving causes constructive/destructive phase interference between STFT frames, producing large transient amplitude spikes.

### PHASE RANDOMISER

Each hop, rotates the phase of bins 1..K−2 by a random amount drawn from a per-bin xorshift64 PRNG:

```
rng ^= rng << 13
rng ^= rng >> 7
rng ^= rng << 17
rand_unit = rng / 2^64   ∈ [0, 1)
rand_phase = (2 × rand_unit − 1) × π × amount

bins[k] = polar(|bins[k]|, arg(bins[k]) + rand_phase)
```

Bins k=0 (DC) and k=K−1 (Nyquist) are skipped — they must remain real-valued for the inverse FFT. The PRNG still advances for these bins to maintain sequence integrity.

### SPECTRAL CONTRAST

See [section 7](#7-spectral-contrast-engine).

---

## 7. Spectral contrast engine

`SpectralContrastEngine` enhances the perceptual contrast of the spectrum by boosting bins that are above their local spectral mean and cutting bins that are below it. It reuses the same `BinParams` infrastructure as the compressor.

### Algorithm (4 passes per hop)

**Pass 1 — Build magnitude prefix sum**

```
mag_prefix[0] = 0
mag_prefix[k+1] = mag_prefix[k] + |bins[k]|
```

All magnitudes come from the unmodified input bins so no bin's computation sees already-modified data from the same frame.

**Pass 2 — Local mean, temporal tracking, gain computation**

For each bin k:

```
width_ratio = 2^(smoothing_semitones / 12)   (min 0.5 semitones enforced by param range)
k_lo = floor(k / width_ratio)
k_hi = ceil(k × width_ratio)

local_mean[k] = (mag_prefix[k_hi+1] − mag_prefix[k_lo]) / (k_hi − k_lo + 1)
```

One-pole temporal low-pass on the local mean (separate attack/release):

```
contrast_env[k] ← one_pole_lp(local_mean[k], attack_ms, release_ms)
```

Deviation of bin k from its smoothed local mean (in dB):

```
env = max(contrast_env[k], 1e−10)
deviation_db = clamp(20 × log₁₀(|bins[k]| / env), −48, +48)
```

The ±48 dB clamp prevents startup transient explosion before `contrast_env` has converged from its zero initial state.

Proportional contrast gain (with optional soft knee):

```
gr_db = deviation_db × (ratio − 1)

Soft knee (knee_db > 0):
  if |deviation_db| ≤ knee_db/2:
    gr_db = deviation_db × (ratio − 1) × (|deviation_db| / knee_db)
  else:
    gr_db = deviation_db × (ratio − 1)   [full gain]
```

At `ratio = 2`: a bin +6 dB above its mean → +6 dB boost; a bin −6 dB below → −6 dB cut.
At `ratio = 0`: all bins pulled toward the local mean (spectrum flattening).
At `ratio = 1`: no effect.

The contrast depth knob maps to ratio via: `ratio = max(0, 1 + depth_db / 6)`.

**Pass 3 — Frequency-domain gain mask smoothing (anti-warbling)**

Smooth the raw per-bin GR values using the same prefix-sum box filter as the compressor:

```
GR_smooth[k] = mean(gr_db[k_lo..=k_hi])
```

This reduces abrupt bin-to-bin gain discontinuities that would otherwise manifest as spectral warbling artifacts in the time domain (the IFFT of a step-function gain mask is a sinc, which introduces pre/post ringing audible as a metallic shimmer).

Note: with the Width param capped at 0.5 semitones, the smoothing window is narrow and does not significantly dilute isolated-peak enhancement. For near-zero Width the smoothing branch is bypassed entirely.

**Pass 4 — Apply gain, makeup, auto-makeup, mix**

```
auto_comp = −auto_makeup_db[k]   (if auto makeup enabled, else 0)
total_db  = clamp(GR_smooth[k] + makeup_db[k] + auto_comp, −80, +40)
linear_gain = 10^(total_db / 20)
bins[k] = bins[k] × ((1 − mix[k]) + mix[k] × linear_gain)
```

The `total_db` clamp prevents f32 overflow when using high ratio values immediately after reset.

**Suppression output**

```
suppression_out[k] = max(0, −GR_smooth[k])
```

Only cuts (negative GR) are currently forwarded to the GUI stalactite display.

---

## 8. Sidechain processing

When a sidechain signal is present on the plugin's auxiliary input:

1. A separate `StftHelper` (`sc_stft`) processes the sidechain at the same hop boundaries.
2. The sidechain complex bins are converted to magnitudes and smoothed with their own attack/release time constants (`SC Attack`, `SC Release`).
3. The smoothed sidechain magnitude vector is passed to `engine.process_bins()` as the `sidechain: Option<&[f32]>` argument.
4. When sidechain is `Some(...)`, the compressor uses the sidechain magnitudes for envelope detection instead of the main signal magnitudes. The main signal's bins are still the ones modified.

**SC Gain** applies a linear dB offset to the sidechain signal before STFT analysis.

When no sidechain input is connected (empty aux bus), `sidechain = None` and the compressor detects from the main signal.

---

## 9. Stereo modes

| Mode | Description |
|------|-------------|
| Linked | Single engine, both channels compressed identically using the averaged magnitude. |
| Independent | Two separate engine instances (`engine` for L, `engine_r` for R). Each channel is processed independently with its own envelope state. |
| MidSide | L/R converted to M/S before STFT (`M = (L+R)/√2`, `S = (L−R)/√2`). Single engine operates on the mid-side representation. Decoded back after STFT synthesis. |

---

## 10. GUI parameter reference

### Curve display

| Element | Description |
|---------|-------------|
| Teal line | Pre-FX peak-hold magnitude (dBFS, normalised to 0 dBFS = 0.0) |
| Pink line | Post-FX magnitude after gain reduction |
| Gradient fill | Area between the two lines, visualising gain reduction |
| Coloured curves | Per-parameter frequency response (dim = inactive, lit = selected) |
| ▶ node (node 0) | Low-shelf node |
| ◀ node (node 5) | High-shelf node |
| Circle nodes | Bell-band nodes (nodes 1–4) |
| Dashed lines | Attack and release curves (to distinguish from continuous curves) |
| Grey line | True time line on attack/release curves showing actual per-bin time after global × Freq scaling |

### All parameters

| Parameter | ID | Default | Range | Description |
|-----------|-----|---------|-------|-------------|
| Input Gain | `input_gain` | 0 dB | ±18 dB | Pre-processing gain |
| Output Gain | `output_gain` | 0 dB | ±18 dB | Post-processing gain |
| Mix | `mix` | 1.0 | 0–1 | Global dry/wet (also curve-shapeable) |
| Attack | `attack_ms` | 10 ms | 0.5–200 ms | Global attack time constant |
| Release | `release_ms` | 80 ms | 1–500 ms | Global release time constant |
| Freq Scale | `freq_scale` | 0.5 | 0–1 | Frequency-dependent time scaling strength |
| SC Gain | `sc_gain` | 0 dB | ±18 dB | Sidechain input level trim |
| SC Attack | `sc_attack_ms` | 5 ms | 0.5–100 ms | Sidechain envelope attack |
| SC Release | `sc_release_ms` | 50 ms | 1–300 ms | Sidechain envelope release |
| Lookahead | `lookahead_ms` | 0 ms | 0–10 ms | Lookahead delay (future use) |
| Stereo Link | `stereo_link` | Linked | Linked/Independent/MidSide | Stereo processing mode |
| Threshold Mode | `threshold_mode` | Absolute | Absolute/Relative | Absolute: fixed dBFS. Relative: threshold tracks the spectral envelope |
| Threshold Slope | `threshold_slope` | 0 dB/oct | ±6 dB/oct | Spectral tilt of threshold around 1 kHz |
| Threshold Offset | `threshold_offset` | 0 dB | ±40 dB | Uniform shift of entire threshold curve |
| Sensitivity | `sensitivity` | 0 | 0–1 | How selectively peaks are targeted (0=absolute, 1=fully relative) |
| Suppression Width | `suppression_width` | 0.2 st | 0–0.5 st | GR mask blur radius in semitones |
| Auto Makeup | `auto_makeup` | off | bool | Long-term GR compensation |
| Delta Monitor | `delta_monitor` | off | bool | Monitor only what is being removed |
| Effect Mode | `effect_mode` | Bypass | Bypass/Freeze/PhaseRand/SpectralContrast | Active effect |
| Phase Rand Amount | `phase_rand_amount` | 0.5 | 0–1 | Phase randomiser depth (fraction of π) |
| Spectral Contrast | `spectral_contrast_db` | 6 dB | −12…+12 dB | Contrast enhancement depth |
| Graph Floor | `graph_db_min` | −100 dB | −160…−20 dB | Spectrum display lower bound (GUI only) |
| Graph Ceil | `graph_db_max` | 0 dB | −20…0 dB | Spectrum display upper bound (GUI only) |
| Peak Falloff | `peak_falloff_ms` | 300 ms | 0–5000 ms | Peak-hold decay time (GUI only) |

### Curve channels

| Index | Name | Neutral gain | Maps to |
|-------|------|-------------|---------|
| 0 | THRESHOLD | 1.0 | −20 dBFS threshold |
| 1 | RATIO | 1.0 | 1:1 compression ratio |
| 2 | ATTACK | 1.0 | Global attack × 1 |
| 3 | RELEASE | 1.0 | Global release × 1 |
| 4 | KNEE | 1.0 | 6 dB soft knee |
| 5 | MAKEUP | 1.0 | 0 dB makeup gain |
| 6 | MIX | 1.0 | 100% wet |

Curves are stored as `[CurveNode; 6]` arrays persisted in the plugin state. Default nodes for every curve produce the neutral value at all frequencies.

---

## Implementation notes

- **Real-time safety:** No allocation, no mutex locking, and no I/O on the audio thread. Triple-buffer channels (`triple_buffer` crate) are used for lock-free GUI↔audio communication. The `assert_process_allocs` feature (enabled in Cargo.toml) aborts the process if the audio thread allocates.
- **Denormal protection:** `flush_denormals()` sets FTZ+DAZ CPU flags at the start of each process block.
- **xorshift64 PRNG:** Used by the phase randomiser. Sequence: `state ^= state << 13; state ^= state >> 7; state ^= state << 17`. Full 2^64−1 period, no allocation.
- **Prefix-sum box filter:** Both the compressor GR smoothing and the contrast local-mean computation use prefix sums for O(N) log-frequency box filtering per hop, regardless of kernel width.
