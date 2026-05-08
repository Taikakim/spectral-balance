# Per-module hidden parameters audit (2026-05-08)

Inventory of internal constants and hardcoded values across every module, classified by whether they're worth exposing for prototyping.

## Categories

- **🎛 Musical** — would benefit from real-time tweaking; expose as curve or slot scalar.
- **🔒 Limit** — memory/safety bound (max sizes for SmallVec, etc.); leave hardcoded.
- **⚙️ Algorithmic** — internal numerical convention (e.g. golden-ratio hash multiplier); leave hardcoded.
- ✅ — already exposed via curve or slot scalar.

## Module-by-module

### Dynamics — fully exposed ✅

6 curves (THRESHOLD/RATIO/ATTACK/RELEASE/KNEE/MIX) + global Atk/Rel/Sens/Width knobs in the Dynamics group panel. No hidden musical params.

### Freeze — fully exposed ✅

5 curves (LENGTH/THRESHOLD/PORTAMENTO/RESISTANCE/MIX). Per-hop accumulator cap in E-1 is now `0.1` per hop — could be exposed as a "RESPONSIVENESS" slot scalar if the 120 ms / resistance unit feels wrong, but musically the 0.1 cap is a reasonable opinion.

### PhaseSmear — mostly exposed

- 3 curves (AMOUNT / PEAK HOLD / MIX).
- 🎛 `std::f32::consts::PI` as the max random-phase scale at amount=2 (`phase_smear.rs:107`). Could expose as a 4th `PHASE_RANGE` curve — at neutral 1.0 reproduces π, at 2.0 gives 2π (full rotations), at 0.5 gives π/2 (subtler smear).

### Contrast — fully exposed ✅

6 curves (THRESHOLD/RATIO/ATTACK/RELEASE/KNEE/MIX) + Atk/Rel/Sens/Width panel — extended 2026-05-08.

### Gain — fully exposed ✅

2 curves (GAIN, PEAK HOLD) + 4 modes (Add/Subtract/Pull/Match) selecting interpretation. PEAK HOLD `gain * 200ms` legacy mapping still on idx 10 (also used by SC Smooth & Punch HEAL). Mid-range PEAK HOLD calibration deferred — needs custom inverse-compose offset_fn.

### Mid/Side — fully exposed ✅

5 curves (BALANCE/EXPANSION/DECORREL/TRANSIENT/PAN). No hidden musical params.

### T/S Split — fully exposed ✅

2 curves (SENSITIVITY/SMOOTHNESS) — extended 2026-05-08. SMOOTHNESS replaces the previously-hardcoded `slow_coeff = 0.98`.

### Harmonic — placeholder

0 curves, no DSP. Module is currently a passthrough that fills `suppression_out` with zeros. Needs design before any exposure makes sense. **Recommendation:** delete from `ASSIGNABLE` until designed, or implement the harmonic-grouping consumer logic.

### Future — partially exposed

5 curves (AMOUNT/TIME/THRESHOLD/SPREAD/MIX), 2 modes (Print-Through/Pre-Echo).

- 🔒 `MAX_ECHO_FRAMES = 64` (memory bound).
- 🎛 PrintThrough leak fraction nominally `5%` at AMOUNT=1.0 — already exposed via AMOUNT curve.

Nothing significant to expose beyond current.

### Punch — partially exposed

6 curves (AMOUNT/WIDTH/FILL_MODE/AMP_FILL/HEAL/MIX), 2 modes (Direct/Inverse).

- 🔒 `MAX_PEAKS = 32`, `MAX_DRIFT_SITES = 64` (memory bounds).
- 🎛 FILL_MODE curve already selects between Gaussian/triangle/boxcar fill kernels — the per-kernel widths could be parametrized but it's diminishing returns.

### Rhythm — exposed (with grid)

5 curves + 8×8 Arpeggiator grid in inline panel + BPM/NoteIn trigger picker (Arpeggiator only). Per-mode tempo subdivisions read from host transport — no hidden musical constants worth exposing.

### Geometry — partially exposed

5 curves, 2 modes (Chladni/Helmholtz).

- 🔒 `N_TRAPS = 8` (Helmholtz max cavities), `GEO_GRID_W = 128`, `GEO_GRID_H = 64` (Chladni plate grid).
- 🎛 Could expose grid resolution as a slot scalar for Chladni, but algorithmic re-init on change is non-trivial. **Skip.**

### Modulate — partially exposed

6 curves + 8 modes + Repel/SC-pos toggles for Gravity.

- 🎛 `zeta = 0.707` (PLL critical damping factor, `modulate.rs:~`) — Gravity uses this as gravity-well shape parameter. Could be a slot scalar "DAMPING" for PLL Tear / Gravity modes.
- 🎛 `mains_hz = 50/60` (Ground Loop) — already selectable via RATE curve mapping.
- 🎛 `harmonics = 1 + reach * 2` (Ground Loop count) — already curve-driven.
- 🎛 `PLL_TEAR_THRESHOLD = π/2` — could expose as TEAR_ANGLE slot scalar for PLL Tear mode.

**Recommendation:** add a `ModulateScalars { damping: f32, tear_angle: f32 }` slot scalar set, exposed as 2 knobs in a Modulate panel widget. Conditionally visible per mode.

### Circuit — partially exposed

5 curves + 10 modes.

- 🎛 `VACTROL_TAU_FAST = 8 ms`, `VACTROL_TAU_SLOW = 250 ms` (Vactrol mode envelope time constants). The fast/slow gap defines the characteristic ringing. **High prototyping value** for tuning the analog feel.
- 🔒 `BBD_STAGES = 4` (BBD bucket count, fixed by the 4-stage analog model — doesn't make sense to vary).
- ⚙️ `KNUTH_GOLDEN = 2654435761` (hash multiplier, internal).

**Recommendation:** add `CircuitScalars { vactrol_fast_ms: f32, vactrol_slow_ms: f32 }` for Vactrol mode, exposed via a 2-knob panel.

### Life — significantly under-exposed 🎛

5 curves + 10 modes. Each mode has a `*_AMOUNT_SCALE` and `*_REACH_SCALE` constant that defines how strongly its curves map to actual DSP behavior. Representative list:

- `VISCOSITY_D_MAX = 0.45` — viscosity amount → diffusion coefficient mapping.
- `SURFACE_TENSION_AMT_MAX = 0.05`, `SURFACE_TENSION_REACH_MAX = 8`.
- `ARCHIMEDES_DUCK_FLOOR = 0.05`, `ARCHIMEDES_CAPACITY_FLOOR = 1e-6`.
- `NON_NEWTONIAN_DISPLACEMENT_CAP = 10.0`.
- `STICTION_DECAY_MIN = 0.05`, `STICTION_DECAY_RANGE = 0.45`.
- `YIELD_HEAL_MIN = 0.005`, `YIELD_HEAL_RANGE = 0.045`, `YIELD_BIAS_CAP = 10.0`.
- `CAPILLARY_AMOUNT_SCALE = 0.025`, `CAPILLARY_AMOUNT_MAX = 0.05`, `CAPILLARY_REACH_SCALE = 16.0`, `CAPILLARY_REACH_MIN/MAX = 1/32`.
- `SANDPAPER_*` scales (5 constants).
- `BROWNIAN_AMOUNT_SCALE = 1.0`, `BROWNIAN_DRIFT_SCALE = 0.1`.

**Recommendation:** each Life mode would benefit from a dedicated `*_MASTER_SCALE` slot scalar that multiplies the per-mode hardcoded `*_AMOUNT_SCALE`. Exposing all is ~10 scalars across modes. **Highest prototyping ROI** of all the multi-mode modules.

### Past — fully exposed ✅

5 curves (incl. SPREAD active in all 5 modes after E-2) + PastScalars panel (`floor_bin`, `window_frames`, `rate`, `dither`).

### Kinetics — significantly under-exposed 🎛

5 curves + 8 modes + WellSource/MassSource sub-pickers in popup.

- 🔒 `MAX_TUNING_FORKS = 16`, `MAX_HARMONIC_SPRINGS = 8`, `MAX_PEAKS = 16` (memory bounds).
- 🎛 `SC_ENVELOPE_TAU_HOPS = 1.0` (Sidechain envelope smoothing for GravityWell/InertialMass).
- 🎛 `SC_MASS_RATE_SCALE = 5.0` (InertialMass sidechain mode strength).
- 🎛 `TUNING_FORK_MIN_SEP = 4` (minimum bin separation between detected forks).
- 🎛 `ORBITAL_SAT_HALF_WINDOW = 16` (Orbital Phase satellite radius).
- 🎛 `ORBITAL_PEAK_THRESHOLD_FACTOR = 2.0` (Orbital peak detection threshold above mean).
- 🎛 `STATIC_WELL_BASELINE = 1.05` (GravityWell static baseline magnitude).
- 🎛 `SC_WELL_THRESHOLD_FRAC = 0.4` (GravityWell sidechain threshold fraction of peak).

**Recommendation:** `KineticsScalars` with the 6 musical constants. Conditionally relevant per mode (Orbital uses ORBITAL_*, GravityWell uses SC_WELL_THRESHOLD_FRAC + STATIC_WELL_BASELINE, etc.).

### Harmony — partially exposed

6 curves + 8 modes + Inharmonic submode picker (Stiffness/Bessel/Prime).

- 🎛 Per-mode magic numbers for chord templates (Chordification), undertone depth ratios, formant shift accuracy, etc. Without a full audit each is hard to identify by name.

**Recommendation:** spot-audit on demand when the user wants a specific tuning.

## Prototyping priority order (recommended)

If you want to expose more, the highest-impact targets in order:

1. **Life** — every mode has 1-3 scale constants that directly shape its character. Adding `LifeScalars` with per-mode scale knobs would unlock the most musical exploration.
2. **Kinetics** — 6 named musical constants across modes.
3. **Circuit Vactrol** — 2 time constants that define the analog feel.
4. **Modulate** — `damping` and `tear_angle` for PLL/Gravity.
5. **PhaseSmear** — `PHASE_RANGE` curve for max random-phase scale.

## Implementation pattern

Use Past's `PastScalars` as the reference template:

1. Define `<Module>Scalars` struct in `src/dsp/modules/<module>.rs`.
2. Add `set_<module>_scalars` to the `SpectralModule` trait.
3. Pipeline reads scalars from params (per-slot params or a central struct) and pushes them into the module each block.
4. Module reads scalars from `self.scalars` in `process()` instead of hardcoded constants.
5. Add panel widget (or inline UI) that renders knobs/dropdowns when this module is selected. PastModule's panel widget at `src/editor/past_panel.rs` is the template.

Per-slot params are added via build.rs codegen (see `slot_arp_grid` etc.). Or use `Arc<Mutex<[<Module>Scalars; 9]>>` if the values don't need host automation (faster to add, no preset persistence).

## Doing nothing is also valid

For final release the user has stated: "we can then hide stuff that is not meaningful to tweak for an user." Many of the above would be exposure for the prototyping phase ONLY. Documenting them here means we know what knobs to add and remove later.
