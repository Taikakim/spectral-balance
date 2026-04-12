# Spectral Balance — User Manual

Spectral Balance is a spectral compressor for Linux/Bitwig Studio. It suppresses resonances, tames harsh frequencies, and controls the spectral balance of a mix — similar in concept to Soothe2 — but built around a familiar parametric EQ-style drawing interface.

---

## Installation

1. Build the plugin:
   ```
   cargo run --package xtask -- bundle spectral_forge --release
   ```
2. Copy the bundle to Bitwig's CLAP path:
   ```
   cp target/bundled/spectral_forge.clap ~/.clap/
   ```
3. Restart Bitwig and rescan plugins. The plugin appears as **Spectral Forge** under CLAP.

> The plugin reports FFT_SIZE (2048 samples) of latency to the host. Bitwig compensates for this automatically in timeline playback.

---

## The Concept

Where a regular compressor tracks the overall signal level, Spectral Balance compresses independently across **1025 frequency bins** (each ~21 Hz wide at 44.1 kHz). Each bin has its own envelope follower, gain computer, and makeup stage.

The 7 **parameter curves** (threshold, ratio, attack, release, knee, makeup, mix) let you sculpt how compression behaves across the frequency spectrum. A flat curve applies the same value everywhere. A bell dip in the threshold curve means compression engages at a lower level in that frequency range — so narrow resonances get caught without touching the rest of the signal.

---

## Controls

### Top row — Parameter Selector

Click a button to choose which parameter curve is shown in the editor:

| Button    | What it controls                                |
|-----------|-------------------------------------------------|
| THRESHOLD | Level (dBFS) at which compression begins        |
| RATIO     | Compression ratio (1:1 = off, 20:1 = limiting)  |
| ATTACK    | How fast gain reduction engages (ms)            |
| RELEASE   | How fast gain reduction releases (ms)           |
| KNEE      | Soft-knee width (0 = hard knee, wide = gentle)  |
| MAKEUP    | Per-bin makeup gain added after compression     |
| MIX       | Dry/wet blend per bin (1.0 = fully wet)         |

### Curve Editor (main area)

The curve editor shows a parametric EQ-style magnitude response that sets how the selected parameter varies across frequency.

- **Neutral position** for all curves is the flat line through the centre: this applies the same value everywhere (set by the global sliders in the control strip).
- **Pulling a node down** in THRESHOLD lowers the threshold — more compression in that band.
- **Pulling a node down** in RATIO lowers the ratio — less compression (more gentle) in that band.
- **Pulling a node up** in MAKEUP adds positive makeup gain to that band.

**Node interaction:**
| Action            | Effect                                          |
|-------------------|-------------------------------------------------|
| Drag node         | Move frequency and gain                         |
| Scroll wheel      | Adjust bandwidth (narrower = sharper Q)         |
| Double-click node | Reset node to neutral (y=0, default Q)          |

Nodes at the far left and right are **shelves**; the four inner nodes are **bells** (Gaussian shape in log-frequency space).

### Background display

- **Spectrum bars** (bottom, blue→green→yellow→red): real-time FFT magnitude of the input signal, log-scaled frequency.
- **Suppression stalactites** (top, dropping downward): per-bin gain reduction applied by the compressor. Longer stalactites = more compression in that bin.

### Control Strip (bottom)

| Knob / Button | Range          | Default | Notes                                                            |
|---------------|----------------|---------|------------------------------------------------------------------|
| IN            | ±18 dB         | 0 dB    | Input gain before STFT (smoothed, 20 ms)                        |
| OUT           | ±18 dB         | 0 dB    | Output gain after STFT (smoothed, 20 ms)                        |
| ATK           | 0.5–200 ms     | 10 ms   | Global attack base time (curve multiplies this per bin)         |
| REL           | 1–500 ms       | 80 ms   | Global release base time (curve multiplies this per bin)        |
| FREQ          | 0–1            | 0.5     | Frequency-dependent timing: 1.0 = bass gets much longer times  |
| MIX           | 0–1            | 1.0     | Global wet/dry (multiplied with per-bin MIX curve)              |
| SC GAIN       | ±18 dB         | 0 dB    | Sidechain input gain                                             |
| AUTO MK       | toggle         | off     | Auto makeup: compensates long-term average gain reduction       |
| DELTA         | toggle         | off     | Delta monitor: outputs dry minus wet (hear what's being removed)|

---

## Stereo Link Modes

Access via the **Stereo Link** parameter (right-click → assign, or use a macro knob):

| Mode        | Behaviour                                                            |
|-------------|----------------------------------------------------------------------|
| Linked      | Both channels share one engine — same GR applied to L and R        |
| Independent | L and R each have their own envelope follower and gain computer     |
| MidSide     | M/S encode before compression, decode after — compress M and S separately |

**MidSide** is useful for controlling low-end weight (mid) and stereo width (side) independently.

---

## Threshold Modes

Access via the **Threshold Mode** parameter:

| Mode     | Behaviour                                                                        |
|----------|----------------------------------------------------------------------------------|
| Absolute | Threshold is a fixed dBFS level. Works like a normal compressor per bin.        |
| Relative | Detection normalised against the local spectral envelope (3-bin median). Only bins that stick out above their neighbours trigger compression — the spectral shape is preserved while resonances are caught. |

**Relative mode** is the "Soothe-like" mode: it leaves the spectral tilt alone and only compresses peaks relative to the local context.

---

## Sidechain

Connect a signal to the sidechain input (Bitwig: add the plugin in a device chain, right-click → Sidechain). The sidechain magnitude per bin drives the envelope follower instead of the main signal. Useful for:

- Ducking a specific frequency range of one instrument when another plays.
- Spectral de-essing driven by a send with only the harsh frequencies boosted.
- Mid/side routing tricks.

The **SC GAIN** knob adjusts the sidechain level before detection.

---

## Typical workflows

### Tame a resonant instrument

1. Put Spectral Balance on the instrument channel.
2. Select **THRESHOLD**, pull the node near the resonance frequency **down** (lower threshold → more compression there).
3. Use **Relative** threshold mode so the plugin only responds to peaks relative to the instrument's own spectral shape.
4. Enable **DELTA** to hear what is being removed. You should hear the resonance in isolation. Disable DELTA when satisfied.

### De-ess a vocal

1. Put Spectral Balance on the vocal.
2. Select **THRESHOLD**, pull down in the 4–10 kHz region.
3. Set a fast **ATK** (1–5 ms) and medium **REL** (50–100 ms).
4. Increase **RATIO** globally or boost it with the RATIO curve in the sibilance range.
5. Use **DELTA** to verify you're catching sibilants, not consonants.

### Spectral glue on a bus

1. Put Spectral Balance on a bus (drums, mix bus).
2. Leave all curves flat (neutral).
3. Use a gentle ratio (2:1–4:1) and moderate attack/release.
4. **Linked** mode for consistent stereo image; **MidSide** if you want to leave the stereo width alone.
5. Enable **AUTO MK** so average loudness is preserved.

### Frequency-targeted sidechain duck

1. Put Spectral Balance on a pad or synth.
2. Route the kick or bass to the sidechain input.
3. Pull down **THRESHOLD** at the frequencies that clash (e.g. 60–200 Hz).
4. Keep other frequencies at neutral threshold — the duck only happens where the sidechain is loud.

---

## Test files (test_flac/)

Included test files for evaluating the plugin:

| File                                           | Contains                                             | What to test                          |
|------------------------------------------------|------------------------------------------------------|---------------------------------------|
| `breakbeat_4030hz_bell-curve-high-q_resonance` | Sharp bell resonance at 4030 Hz                      | Narrow threshold dip at 4 kHz        |
| `breakbeat_kick_resonance`                     | Kick with ring/resonance artefact                    | Low-mid threshold dip, fast attack   |
| `breakbeat_sweep-high-q-200hz_to_4khz_resonance` | Sweeping resonance 200 Hz → 4 kHz                  | Relative mode tracking a moving peak |
| `chord-brillant-resonance-attack-decay-sweep`  | Chord with attack/decay resonance sweep              | Attack/release curve shaping         |
| `saw_filter_decay`                             | Sawtooth with filter decay (spectral change over time) | Temporal response of envelope follower |

Load these into Bitwig on a track, insert Spectral Balance, and enable **DELTA** while adjusting the threshold curve to isolate the resonance you want to remove.

---

## Technical notes

- **Latency:** FFT_SIZE = 2048 samples (reported to host, compensated automatically in Bitwig).
- **Sample rates:** Any rate supported by the host. Timing values (attack/release) are compensated for sample rate.
- **FFT overlap:** 75% (4× overlap), Hann window, Hann² OLA normalisation. Reduces smearing artefacts.
- **Frequency resolution:** ~21 Hz per bin at 44.1 kHz (1025 bins from 0 to Nyquist).
- **Bin linking:** Gain reduction is 3-tap weighted-averaged across adjacent bins (w=0.5/0.25/0.25) to prevent narrow zipper artefacts.

---

## Known limitations

- Linux and CLAP only. No Windows, macOS, VST3, or AU support.
- The lookahead parameter is reserved for a future implementation; currently the STFT latency provides effective transient anticipation.
- The GUI requires OpenGL (egui/wgpu via nih-plug-egui). Some headless environments won't open the editor.
