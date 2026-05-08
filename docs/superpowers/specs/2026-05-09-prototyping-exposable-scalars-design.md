# Prototyping-Exposable Scalars — Design

**Status:** APPROVED — ready for plan.
**Date:** 2026-05-09
**Audit predecessor:** `docs/superpowers/2026-05-08-prototyping-exposable-params.md`

## Goal

Expose the highest-ROI hardcoded musical constants in five experimental
modules as host-automatable per-slot `FloatParam`s, gated for visibility
behind the `dev-build` feature flag. At default values every exposed
scalar reproduces the current hardcoded behaviour exactly, so existing
patches are bit-identical. Values persist in presets even on release
builds — only the editing UI is gated.

## Non-goals

- Re-architecting any module's DSP. Scalars replace literals one-for-one.
- Exposing memory bounds (`MAX_PEAKS`, `BBD_STAGES`, etc.); those stay hardcoded.
- Hiding params from the host post-release. Future work; out of scope.
- Harmony tuning constants (audit calls them "spot-audit on demand").
- PAST, Geometry, Future, Punch, Rhythm — already adequately exposed per audit.

## Architectural pattern

Reference template: `PastScalars` + `past_panel.rs` + per-slot params via
`build.rs` codegen.

For each module M ∈ {Life, Kinetics, Circuit, Modulate}:

1. Define `MScalars` struct in `src/dsp/modules/<m>.rs` with `safe_default()`
   that returns the current hardcoded values bit-exactly.
2. Trait method `set_<m>_scalars(&mut self, scalars: MScalars)` on
   `SpectralModule`, default no-op impl in `src/dsp/modules/mod.rs`.
3. `FxMatrix` gets `set_<m>_scalars(&mut self, scalars: &[MScalars; 9])`
   that fans out to each slot's module instance.
4. `Pipeline::process()` reads the 9 per-slot scalar sets from params each
   block, packs them into `[MScalars; 9]`, calls `fx_matrix.set_<m>_scalars(...)`.
5. `build.rs` emits the `s{slot}_<m>_<scalar>: FloatParam` fields and a
   per-slot dispatch macro. Same pattern as the existing
   `s{s}_past_floor_hz` / `s{s}_past_reverse_window_s` etc.
6. Each module M has a panel widget `src/editor/<m>_panel.rs` that renders
   the relevant scalars for the current mode. **The whole panel module is
   `#[cfg(feature = "dev-build")]`** — non-dev builds compile the file out
   and `module_spec(M).panel_widget` resolves to `None`.
7. PhaseSmear is the exception: it adds a 4th curve `PHASE_RANGE` instead
   of a scalar set. Implementation detail in §6.

PhaseSmear's curve does not need any of the panel scaffolding above — it's
a regular curve that shows/hides like the other PhaseSmear curves.

## 1. Life — `LifeScalars`

Per-mode `MASTER_SCALE` multipliers. Each scalar is a linear `0.0..2.0`
FloatParam with default `1.0`. At default, behavior is unchanged. At
`0.0`, that mode's effect is multiplied to nil and the mode degenerates
to passthrough. At `2.0`, the mode runs at twice the hardcoded scale.

| Field | Multiplies | Default | Range |
|---|---|---|---|
| `viscosity_scale` | `VISCOSITY_D_MAX = 0.45` | 1.0 | 0..2 |
| `surface_tension_scale` | `SURFACE_TENSION_AMT_MAX = 0.05` (REACH stays integer) | 1.0 | 0..2 |
| `non_newtonian_scale` | `NON_NEWTONIAN_DISPLACEMENT_CAP = 10.0` | 1.0 | 0..2 |
| `stiction_scale` | `STICTION_DECAY_RANGE = 0.45` | 1.0 | 0..2 |
| `yield_scale` | `YIELD_HEAL_RANGE = 0.045` (BIAS_CAP stays a hard cap) | 1.0 | 0..2 |
| `capillary_scale` | `CAPILLARY_AMOUNT_SCALE = 0.025` | 1.0 | 0..2 |
| `sandpaper_scale` | `SANDPAPER_AMOUNT_SCALE = 0.05` | 1.0 | 0..2 |
| `brownian_scale` | `BROWNIAN_AMOUNT_SCALE = 1.0` | 1.0 | 0..2 |

**Excluded modes:** Crystallization has no `*_AMOUNT_SCALE`-style constant
(the kernel multiplies curves directly with `sustain_envelope`); Archimedes
mode's `DUCK_FLOOR` has inverted multiplier semantics (multiplying it
weakens the effect rather than scaling it). For both, the panel renders
nothing in mode-conditional view.

**Wire-in:** the `*_AMOUNT_SCALE` constants are read inside per-mode kernels.
Each kernel multiplies its constant by `self.scalars.<mode>_scale` once at
the top of the kernel; no other change.

8 scalars × 9 slots = **72 new FloatParams**.

## 2. Kinetics — `KineticsScalars`

7 named scalars, mode-conditional in the panel. All default to current values.

| Field | Replaces | Default | Range | Modes |
|---|---|---|---|---|
| `sc_envelope_tau_hops` | `SC_ENVELOPE_TAU_HOPS = 1.0` | 1.0 | 0.5..4.0 (linear) | GravityWell, InertialMass |
| `sc_mass_rate_scale` | `SC_MASS_RATE_SCALE = 5.0` | 5.0 | 0.5..10.0 (linear) | InertialMass |
| `tuning_fork_min_sep` | `TUNING_FORK_MIN_SEP = 4` | 4 | 1..16 (FloatParam, `as usize` at read) | TuningFork |
| `orbital_sat_half_window` | `ORBITAL_SAT_HALF_WINDOW = 16` | 16 | 4..32 (FloatParam, `as usize`) | OrbitalPhase |
| `orbital_peak_threshold_factor` | `ORBITAL_PEAK_THRESHOLD_FACTOR = 2.0` | 2.0 | 1.0..5.0 (linear) | OrbitalPhase |
| `static_well_baseline` | `STATIC_WELL_BASELINE = 1.05` | 1.05 | 1.0..2.0 (linear) | GravityWell (Static source) |
| `sc_well_threshold_frac` | `SC_WELL_THRESHOLD_FRAC = 0.4` | 0.4 | 0.1..0.9 (linear) | GravityWell (Sidechain source) |

**Note:** `tuning_fork_min_sep` and `orbital_sat_half_window` use FloatParam
with integer-typed reads (`as usize`). Reasoning: keeping them
floating-point in the param surface makes them automatable as a smooth
sweep; the kernel rounding produces step-changes that are musically
correct for these counts. Alternative (IntParam) is more pedantic but
breaks the homogeneous "all scalars are FloatParams" pattern and breaks
build.rs codegen which currently emits FloatParam only.

7 scalars × 9 slots = **63 new FloatParams**.

## 3. Circuit — `CircuitScalars`

2 direct values for Vactrol mode only.

| Field | Replaces | Default | Range |
|---|---|---|---|
| `vactrol_fast_ms` | `VACTROL_TAU_FAST = 0.008 s` (8 ms) | 8.0 | 1.0..50.0 (linear, ms) |
| `vactrol_slow_ms` | `VACTROL_TAU_SLOW = 0.250 s` (250 ms) | 250.0 | 50.0..1000.0 (linear, ms) |

**Note:** the existing constants are in seconds; the param is in
milliseconds (consumes the same UI/format style as the existing PAST
window seconds vs millisecond drag-values). Kernel reads `* 1e-3` to
convert. Default 8.0 ms / 250.0 ms reproduces current behaviour exactly.

2 scalars × 9 slots = **18 new FloatParams**.

## 4. Modulate — `ModulateScalars`

2 direct values, mode-conditional.

| Field | Replaces | Default | Range | Modes |
|---|---|---|---|---|
| `damping` | `zeta = 0.707` (PLL Tear PLL critical-damping factor) | 0.707 | 0.1..2.0 (linear) | PllTear only |
| `tear_angle_rad` | `PLL_TEAR_THRESHOLD = π/2` | π/2 ≈ 1.5708 | π/8..π (linear, radians) | PllTear only |

The existing local `let zeta = 0.707_f32;` at `modulate.rs:462` becomes
`let zeta = self.scalars.damping;`. `PLL_TEAR_THRESHOLD` becomes
`self.scalars.tear_angle_rad`. GravityPhaser was listed in the audit
as a `zeta` consumer but the code grep confirms only PllTear uses it
— GravityPhaser has its own `0.95` momentum-decay constant which the
audit did not flag and which we are not exposing here.

2 scalars × 9 slots = **18 new FloatParams**.

## 5. PhaseSmear — `PHASE_RANGE` curve

PhaseSmear extends from 3 curves to **4**. New curve at index 3:

- Label: `PHASE_RANGE`
- `num_curves(): 3 → 4`
- `ModuleSpec.curve_labels` array gets the new entry
- DSP: replace the existing literal `std::f32::consts::PI` at `phase_smear.rs:107`
  with `curves.get(3).and_then(|c| c.get(k)).copied().unwrap_or(1.0) * std::f32::consts::PI`
- At per-bin curve gain 1.0 (default flat curve): `1.0 × π = π`, current behavior.
- Curve gain 2.0: `2π` (full rotations possible per bin).
- Curve gain 0.5: `π/2` (subtler smear).
- Curve gain 0.0: `0` (no smearing for that bin — bypass-per-bin).

Calibration entry: `phase_smear_config(3, _)` returns a curve display config
with `y_label: "× π"`, `y_min: 0.0`, `y_max: 2.0`, `y_natural: 1.0`,
`grid_lines: [(0.5, "0.5×π"), (1.0, "π"), (1.5, "1.5×π"), (2.0, "2×π")]`,
`offset_fn: off_amount_200`, `natural_at_max: false`. `off_amount_200`
already exists — it's the calibration used for `AMOUNT 0..200%`.

No scalars, no panel widget, no build.rs change. Just an additional
curve channel that fits into the existing curve machinery.

## 6. Visibility / dev-build gating

The pattern: **params always exist; UI controls only render in dev-build.**

```rust
// src/editor/life_panel.rs
#[cfg(feature = "dev-build")]
pub fn draw(ui: &mut Ui, params: &SpectralForgeParams,
            setter: &ParamSetter<'_>, slot: usize) { /* knobs */ }

// src/dsp/modules/mod.rs — module_spec(ModuleType::Life)
panel_widget: {
    #[cfg(feature = "dev-build")]
    { Some(crate::editor::life_panel::draw as PanelWidgetFn) }
    #[cfg(not(feature = "dev-build"))]
    { None }
},
```

Consequence:
- Production build (`cargo build --release`): no panel for Life/Kinetics/
  Circuit/Modulate; the FloatParams still appear in the host's parameter
  list and are still automatable / preset-saveable. The host's plain
  generic UI exposes them. Only the in-plugin curated panel is hidden.
- Dev build (`cargo build --release --features dev-build`): panel shows
  the mode-conditional knobs.

Future cleanup pass — unused scalars become `_unused` shims and eventually
deleted; useful ones graduate to always-visible UI panels. That's a
separate exercise; this design only covers exposure.

PhaseSmear's `PHASE_RANGE` curve is **not** gated. It's a real per-bin
musical control, not a tuning knob; it makes sense to ship it as a
permanent curve.

## 7. Default-correctness invariant

For each module M, this property MUST hold:

> For any input audio + curve state, the output of M with `MScalars =
> MScalars::safe_default()` is bit-identical to the output of M before
> this design landed.

The implementation pattern guarantees this if every per-mode kernel
multiplies in the new scalar (default 1.0 for multipliers, default =
hardcoded for direct values) at exactly the spot where the literal lived.

## 8. Test strategy

For each new scalar set:

1. **Default-correctness test** — `MScalars::safe_default()` matches each
   hardcoded constant exactly. Assert each field. Catches drift if
   someone bumps the hardcoded value without bumping the scalar default.

2. **Pipeline plumbing test** — feed a slot with a non-default scalar set
   via the param surface; `fx_matrix.test_<m>_scalars(slot)` round-trips
   the value. Mirrors the existing `test_past_scalars` helper.

3. **Per-mode behavior test (light)** — for each mode that exposes a
   scalar, drive one block with `scalar=0.0` (or the minimum) and one
   block with `scalar=2.0` (or maximum), assert that output magnitudes
   move in the expected direction. Skip if a mode has no scalars to
   test (e.g. Life modes without their own constants — none of the 10
   are like that).

4. **PhaseSmear PHASE_RANGE curve calibration test** — already covered
   by `tests/curve_calibration_matrix.rs` once the new entry lands in
   `phase_smear_config`. The matrix asserts WYSIWYG round-trip
   automatically across all `(module, curve_idx)` pairs.

5. **No-allocation regression** — `assert_process_allocs` is on for the
   audio thread; any test that runs `Pipeline::process()` already
   catches per-block allocations. Don't add separate alloc tests; rely
   on the existing infrastructure.

## 9. Implementation staging

**Each module is independent.** Recommended order matches audit ROI:

1. **Life** (highest ROI, ~10 scalars). Establishes the pattern in the
   richest case. Once Life lands cleanly, the rest are mechanical
   copies.
2. **Kinetics** (7 scalars across modes — exercises the mode-conditional
   panel).
3. **Circuit Vactrol** (2 scalars — the simplest non-PhaseSmear case).
4. **Modulate** (2 scalars).
5. **PhaseSmear PHASE_RANGE** (curve, no panel scaffolding — completely
   different code path).

Each lands as its own commit (or its own small set of commits — define
struct, wire pipeline, add panel, add tests).

## 10. Param count budget

New FloatParams added: 72 + 63 + 18 + 18 = **171**.

Plus PhaseSmear gets one more curve channel (uses existing `slot_curve_cache[s][3]`,
no new params).

Existing param count is well into the hundreds (117 matrix cells, 9 ×
6 × 3 = 162 per-curve transforms, plus per-slot scalars and module
modes). +189 brings the total comfortably under typical CLAP host
limits but visibly increases the host's param list. This is acceptable
during prototyping; the cleanup pass post-prototyping removes the unused
ones (or demotes them to inert constants).

## 11. Files to create or modify

**Create:**
- `src/editor/life_panel.rs`
- `src/editor/kinetics_panel.rs`
- `src/editor/circuit_panel.rs`
- `src/editor/modulate_panel.rs`

**Modify:**
- `src/dsp/modules/life.rs` (struct + reads)
- `src/dsp/modules/kinetics.rs` (struct + reads)
- `src/dsp/modules/circuit.rs` (struct + reads)
- `src/dsp/modules/modulate.rs` (struct + reads)
- `src/dsp/modules/phase_smear.rs` (curve count → 4, read curve idx 3)
- `src/dsp/modules/mod.rs` (trait method defaults + ModuleSpec wiring)
- `src/dsp/fx_matrix.rs` (4 × set_<m>_scalars dispatchers + 4 × test_<m>_scalars helpers)
- `src/dsp/pipeline.rs` (4 × per-block gather + dispatch)
- `src/params.rs` (4 × accessor helpers)
- `src/editor/mod.rs` (add panel modules)
- `src/editor/curve_config.rs` (PhaseSmear curve_idx=3 entry)
- `build.rs` (codegen for new params)
- `tests/module_trait.rs` or new `tests/scalar_*.rs` files for default-correctness + plumbing tests
- `tests/curve_config.rs` (PhaseSmear 4th-curve assertions)

**Build feature gates:** every panel file is `#[cfg(feature = "dev-build")]`
at the file level. The dispatch in `module_spec(M).panel_widget` is
`#[cfg]`-gated to either `Some(...)` or `None`.

## 12. Open issues / future work (NOT addressed here)

- Hiding the new params from the host post-prototyping. Will need a
  `nih_plug` mechanism for "internal-only" params or a doc-only convention.
- Harmony module tuning constants. Spot-audit only when a specific need arises.
- PAST RESPONSIVENESS scalar (audit §Freeze suggested it as
  optional). Skipped unless user reports a feel issue.
