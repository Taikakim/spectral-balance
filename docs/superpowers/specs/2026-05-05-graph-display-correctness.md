# Graph Display Correctness — Audit & Recalibration

**Status:** SPEC (2026-05-05)

**Goal:** Bring every curve in every module onto the global config-driven UI
system, fix the WYSIWYG calibration mismatch in the offset slider ↔ curve
baseline, and codify per-mode curve visibility for multi-mode modules. The
output is an audit table covering all 18 modules and a fix list with one
implementation task per row that needs work.

**Context:** Tasks 1–9 of `2026-05-05-curve-display-axis-config.md` switched
`gain_to_display(9)` from a linear formula to a logarithmic one and made
`physical_to_y`/`screen_y_to_physical` config-driven. That work missed the
matching calibration of `offset_fn` for log-axis curves: the slider formatter
displays an offset value (e.g. "-36.2 dBFS") that does not match where the
curve baseline actually renders (e.g. -80 dBFS, clamped). A unit test pinpoints
the discrepancy at 43.8 dBFS for one canonical case. The same oversight likely
affects other curves; the audit will surface them.

User-visible symptoms motivating the spec:
- Freeze and PAST THRESHOLD curves stop rendering below ~-40 dBFS even though
  the displayed axis runs to -80 dBFS.
- Future module curves don't redraw on node moves (suspected: missing
  `active_layout` or mode-byte plumbing).
- "Some `[-1, 1]` slider value is being treated as `[0, 1]` somewhere"
  (user's framing — the actual root cause is the additive offset_fn vs
  the logarithmic gain→dBFS).

---

## 1. Strategy and deliverables

The spec produces two artifacts:

1. **Audit table** — `docs/superpowers/specs/2026-05-05-graph-display-audit-table.md`,
   one row per `(ModuleType, raw_curve_idx)` pair (~80–100 rows). Each row
   carries a green/red verdict on calibration consistency, per-mode visibility
   completeness, and "uses global UI system." Red rows are inputs to the
   implementation plan.

2. **Design spec** — this document, which states the architectural rules and
   fix patterns. The implementation plan (next stage) walks the table and
   produces one task per red row plus a small number of global-infrastructure
   tasks.

The implementation plan that follows this spec will have three categories of
tasks:

- **Calibration fix tasks** (one per `WYSIWYG?=red` row) — narrow, mechanical,
  each with a unit test added to the matrix.
- **Per-mode visibility tasks** (one per multi-mode module that needs an
  `active_layout` defined or extended).
- **Global-system migration tasks** (one per original module found bypassing
  `CurveDisplayConfig`).

Plus 1–2 global-infrastructure tasks for `runtime_anchors` extensions
discovered during the audit (e.g., adding `global_attack_ms` substitution).

---

## 2. Audit table schema

The table is the source of truth for the fix list. Columns:

| Column | Source | Purpose |
|---|---|---|
| Module | `ModuleType` variant | Identifier |
| Curve | `spec.curve_labels[i]` | Human label |
| Curve idx | `i` (0..num_curves-1) | Module-local index |
| Display idx | `display_curve_idx(module, i, gain_mode)` | Canonical mapping (0–13) |
| Axis | `cfg.y_log` + shape of `gain_to_display(idx, ...)` | "linear in gain", "log in gain", "linear in display only" |
| offset_fn | `cfg.offset_fn` | Calibration function name |
| WYSIWYG? | `check_wysiwyg(...)` at v ∈ {-1, -0.5, 0, +0.5, +1} | green ✓ / red ✗ |
| Visible in modes | per-mode `active_layout(mode_byte).active` | Which modes show this curve |
| Global system? | grep audit (see §5) | green ✓ / red ✗ + brief note |
| Notes / fix needed | summary | Pinpoints the implementation task |

The `WYSIWYG?` check is mechanical — see Appendix A for the helper that
populates it automatically. The audit is table-generation, not manual
inspection.

---

## 3. Calibration fix patterns

The bugs the audit will surface fall into a small number of categories. Each
has a known fix shape.

### 3.1 Log-gain dBFS thresholds (display indices 0 and 9)

**Symptom:** offset_fn is additive (`g + k·o`), but `gain_to_display` is
logarithmic (`-20 + log10(g)·66.667`). Slider value (linear lerp in display
space, per spec §2) and curve baseline diverge.

**Fix for idx 9 (Freeze threshold, fixed range -80…0, neutral -20):** replace
the additive `off_freeze_thresh` with a multiplicative form whose log produces
the spec lerp.

```rust
pub fn off_freeze_thresh(g: f32, o: f32) -> f32 {
    // Spec §2 lerp targets:
    //   v ≥ 0: display = -20 + 20·v (range -20..0)
    //   v < 0: display = -20 + 60·v (range -80..-20)
    // Inverse of gain→dBFS (`-20 + log10(g)·66.667`):
    //   g = 10^((display + 20) / 66.667)
    //   v ≥ 0:  factor = 10^(0.3·v) (≈ 2^v)
    //   v < 0:  factor = 10^(0.9·v)
    if o >= 0.0 { g * 10f32.powf(0.3 * o) } else { g * 10f32.powf(0.9 * o) }
}
```

**Fix for idx 0 (Dynamics threshold, range `db_min`…`db_max`, neutral -20):**
same shape, but the negative-branch exponent depends on `db_min` (runtime).
Bake the spec's canonical `db_min = -60`:

```rust
pub fn off_thresh(g: f32, o: f32) -> f32 {
    // Calibrated for db_min = -60, db_max = 0. Users who customise db_min
    // get a slight WYSIWYG drift in the negative half; see §7.4.
    if o >= 0.0 { g * 10f32.powf(0.3 * o) } else { g * 10f32.powf(0.6 * o) }
}
```

The 0.6 negative exponent comes from `(y_natural - y_min) / 66.667 = 40/66.667
≈ 0.6` for the canonical -60 dBFS lower bound.

### 3.2 Multiplicative-time axes (display indices 2, 3 — attack/release ms)

**Symptom:** `cfg.y_natural` is hardcoded to `1.0`, but `gain_to_display`
multiplies by `global_attack_ms` (runtime). Slider says "1 ms" at neutral but
the actual displayed curve sits at `global_attack_ms` (e.g. 10 ms).

**Fix:** extend `runtime_anchors` to substitute `y_natural` for indices 2 and
3 from `global_attack_ms` and `global_release_ms`. Existing `off_atk_rel`
(multiplicative `g · 1024^o`) is structurally correct — the bug is that the
slider's anchors don't substitute. Call sites of `runtime_anchors` get the new
arguments threaded.

### 3.3 Linear-gain linear-display axes (display indices 6, 7, 11)

**Symptom:** likely none. `g · k = display`, additive offset is linear in
display. WYSIWYG holds by construction. Audit verifies and checks the box.

If the audit flags any here, the cause is a wrong constant — e.g.,
`off_amount_200` produces gain in [0, 2] but `cfg.y_max` says 200 instead of
100. Trivial fix.

### 3.4 Log-time axes with non-trivial neutral (display indices 8, 10)

**Symptom:** `g · 500 ms = display` (idx 8) is linear in gain, but the axis is
log-rendered. `y_natural=500` and `off_freeze_length(g, o) = g · 8^o` should
be consistent. Audit verifies.

### 3.5 Special cases

Each gets a row in the audit. Idx 13 (PAST Age/Delay) is known broken and
deferred (§7.2). Other indices: ratio (idx 1), knee (idx 4), makeup/dB (idx 5,
12) — verify with `check_wysiwyg`, fix any flagged.

### 3.6 Out-of-spec node range

`curve_widget`'s drag clamp allows `node.y ∈ [-2, +2]` (50% visual headroom
beyond rect). Existing behavior, preserved as-is. Mention in spec; do not
modify.

---

## 4. Per-mode curve visibility

Infrastructure exists (`CurveLayout::active`) and PAST already uses it. Two
gaps to close.

### 4.1 Mode-byte plumbing for all multi-mode modules

`editor_ui.rs:488-497` only consults `slot_past_mode`; other multi-mode
modules fall through to `mode_byte = 0`. Extend the match to cover every
multi-mode module:

```rust
let mode_byte: u8 = match editing_type {
    ModuleType::Past     => params.slot_past_mode.lock()[editing_slot]   as u8,
    ModuleType::Future   => params.slot_future_mode.lock()[editing_slot] as u8,
    ModuleType::Circuit  => params.slot_circuit_mode.lock()[editing_slot] as u8,
    ModuleType::Life     => params.slot_life_mode.lock()[editing_slot]   as u8,
    ModuleType::Modulate => params.slot_modulate_mode.lock()[editing_slot] as u8,
    ModuleType::Rhythm   => params.slot_rhythm_mode.lock()[editing_slot] as u8,
    ModuleType::Punch    => params.slot_punch_mode.lock()[editing_slot]  as u8,
    ModuleType::Harmony  => params.slot_harmony_mode.lock()[editing_slot] as u8,
    ModuleType::Geometry => params.slot_geometry_mode.lock()[editing_slot] as u8,
    _ => 0u8,
};
```

The audit's "Visible in modes" column tells us which modules need this. The
analogous per-block snapshot in `pipeline.rs` (audio thread) gets the same
update so the cache invalidation in `editor_ui.rs` keys correctly.

### 4.2 Per-module `active_layout` definitions

For each multi-mode module, define (or extend) the `active_layout` function
that returns the right `CurveLayout` per mode. The "active" list comes from
inspecting each kernel's parameter signature — if the DSP kernel doesn't
consume a curve in that mode, the curve is hidden.

Example for PAST (already correct, illustrative):

```rust
pub fn active_layout(mode_byte: u8) -> CurveLayout {
    match PastMode::from_u8(mode_byte) {
        PastMode::Granular    => CurveLayout { active: &[0,1,2,3,4], ... },
        PastMode::DecaySorter => CurveLayout { active: &[0,2,4],     ... },
        PastMode::Convolution => CurveLayout { active: &[0,1,2,4],   ... },
        PastMode::Reverse     => CurveLayout { active: &[0,2,4],     ... },
        PastMode::Stretch     => CurveLayout { active: &[0,4],       ... },
    }
}
```

Modules where `active_layout` is `None` but DSP varies by mode get a function
added in the implementation plan.

### 4.3 `editing_curve` snap

Already correct (`editor_ui.rs:514-518`). When `active` changes and the
current `editing_curve` becomes hidden, snap to the first visible. No code
change; verify in tests.

### 4.4 Parameter persistence on hide

Hidden curves' underlying `s{slot}c{curve}_*` params retain their values.
Switching modes back re-exposes them with the user's prior settings.
**Explicit non-goal:** zeroing or disabling hidden params on hide. That would
break automation and preset state across mode switches.

---

## 5. Global UI system compliance

For the original 8 modules (Dynamics, Freeze, PhaseSmear, Contrast, Gain,
MidSide, TsSplit, Harmonic), the audit confirms they're on the new
config-driven path.

### 5.1 Required: `curve_config.rs` arm exists

`curve_display_config(module, curve_idx, gain_mode)` returns a real,
calibrated `CurveDisplayConfig` (not `default_config()`) for every
`(module, curve_idx)` the module declares in `spec.num_curves`.

### 5.2 Required: `display_curve_idx` arm exists

`display_curve_idx(module, curve_idx, gain_mode)` maps to a valid display
index (0–13). Fallthrough to raw `curve_idx` (`_ => curve_idx`) is suspect for
any module not explicitly enumerated — flag in audit.

### 5.3 Forbidden: hardcoded display ranges or transforms

In `src/dsp/modules/*.rs` or `src/editor/*.rs` (other than `curve.rs` and
`curve_config.rs`), forbid:

- Numeric literals like `-60.0`, `-80.0`, `1024.0` near `dB`/`dBFS`/`ms`
  string contexts.
- Local `linear_to_y` / `log_to_y` calls.
- Local match arms on `display_idx` for visual purposes.
- Module-specific clamp values that should live in cfg.

### 5.4 Forbidden: divergent paths between audio and GUI

`pipeline.rs::apply_curve_transform` (audio) and
`curve.rs::apply_curve_adjustments` (GUI) MUST go through the same
`cfg.offset_fn`. Today this holds — the spec mandates it stays that way.

### 5.5 Recommended: paint helpers consume cfg

`paint_grid`, `paint_hover_text`, and `paint_response_curve` consume
`&CurveDisplayConfig` (Tasks 7–8). Verify no caller bypasses these.

The audit's "Global system?" column gets a single ✓/✗ per row with notes
pinpointing any violation.

---

## 6. Testing strategy

### 6.1 Automated WYSIWYG matrix test

`tests/curve_calibration_matrix.rs`: loops over every `(ModuleType,
curve_idx)` pair and runs `check_wysiwyg` (Appendix A) at v ∈ {-1, -0.5, 0,
+0.5, +1}. Failures print module + curve + v + expected/actual.

Rows the audit deferred (e.g., idx 13 history) get `#[ignore]` with an inline
reference to the deferring spec. The test becomes the regression guard.

### 6.2 Per-mode visibility tests

One test per multi-mode module, asserting that for each mode value the
`active_layout(mode_byte).active` list matches the curves the DSP kernel
actually consumes. Pattern:

```rust
#[test]
fn past_active_layout_per_mode_matches_kernel_signature() {
    for mode in PastMode::all() {
        let layout = past::active_layout(mode as u8);
        let expected = match mode {
            PastMode::Granular   => vec![0,1,2,3,4],
            PastMode::DecaySorter=> vec![0,2,4],
            // ...
        };
        assert_eq!(layout.active, expected.as_slice(),
            "PAST mode {:?} active list drift", mode);
    }
}
```

Fails loud if a kernel adds/removes a parameter and the layout isn't
synchronised.

### 6.3 `editing_curve` snap test

Extend `tests/curve_layout.rs` with a test that walks each multi-mode module
through every mode and confirms `editing_curve` lands on the first visible
when it would otherwise be hidden.

### 6.4 Global-system grep checks

`tests/global_system_grep.rs`: a single test that uses `std::process::Command`
to run grep over `src/dsp/modules/*.rs` and asserts on output. Targets:

- `linear_to_y\b|log_to_y\b` outside `curve.rs` → must return zero hits.
- `curve_idx\s*==\s*[0-9]+` in display contexts → must return zero hits.
- `"\s*dBFS\s*"|"\s*dB/oct\s*"` outside `curve_config.rs` → review hits.

Cheap; catches regressions where someone adds a new module with local logic.

### 6.5 Manual smoke test (final gate)

Bundle and load in Bitwig. For every module:

- Drag the offset slider and visually confirm the curve baseline matches the
  slider's displayed value.
- Switch through every mode and confirm visible curves change correctly.
- Move a node and confirm the curve redraws.

Final manual gate before the audit is declared done.

---

## 7. Out of scope

### 7.1 PAST DSP completion (sub-project B)

AMOUNT plumbing across all 5 modes, SMEAR semantics, and other parameter
pass-through gaps belong to the separate "PAST DSP completion" brainstorm.
This spec only declares the **contract** (which curves are active in which
mode) — it does not fix the DSP. The visibility table here is the input
to (B).

### 7.2 PAST Age/Delay (display idx 13)

Known broken: renders flat zero because `total_history_seconds` isn't plumbed
end-to-end. Tracked as Task 14 of `past-module-ux-overhaul` (separate). The
audit row stays red with a "deferred to past-module-ux Task 14" note.

### 7.3 Approach C (phys-space offset architecture)

The bigger refactor where `gain_to_phys` / `phys_to_gain` replace `offset_fn`
entirely. Recorded as a possible future direction; not done now.

### 7.4 Custom `db_min` calibration for idx 0

Section 3.1 bakes `db_min = -60` into `off_thresh`. Users who customise
`db_min` get a slight WYSIWYG drift in the negative half (proportional to the
ratio of actual `db_min` to canonical `-60`). If this matters in practice, a
follow-up plan adds anchors-aware wrappers.

### 7.5 Node parameter clamping `[-2, +2]`

The drag clamp in `curve_widget` allowing `node.y` past visual rect is
preserved as-is. UX decisions about whether to tighten it are out of scope.

### 7.6 Curve label capitalisation

ALL_CAPS vs Word Caps inconsistency in module `curve_labels` is cosmetic;
separate small follow-up.

### 7.7 Spectrum / suppression rendering

Other painters in `curve.rs` (`paint_spectrum_and_suppression`,
`paint_peak_hold_envelope_overlay`) have their own dB scaling. Out of scope
unless audit finds they bypass `cfg`.

---

## 8. Deliverables and file structure

| File | Purpose |
|---|---|
| `docs/superpowers/specs/2026-05-05-graph-display-correctness.md` | This spec |
| `docs/superpowers/specs/2026-05-05-graph-display-audit-table.md` | Audit table (filled during implementation) |
| `tests/curve_calibration_matrix.rs` | Automated WYSIWYG matrix |
| `tests/global_system_grep.rs` | Compliance grep tests |
| `src/editor/curve_config.rs` | Recalibrated `off_thresh`, `off_freeze_thresh`, plus any other flagged offset_fns |
| `src/editor/curve.rs` | Extended `runtime_anchors` (attack/release substitution); call-site updates |
| `src/editor_ui.rs` | Mode-byte match for all multi-mode modules |
| `src/dsp/modules/<module>.rs` | New/extended `active_layout` for multi-mode modules |
| `src/dsp/pipeline.rs` | Mode-byte snapshot per slot for the audio thread |

Each `<module>.rs` change is local to that module's `active_layout` function
plus any imports.

---

## Appendix A — `check_wysiwyg` helper

The mechanical calibration check used by the audit and the regression test:

```rust
/// Returns Ok(()) if the offset slider's displayed value matches where the
/// curve baseline actually renders, at v ∈ {-1, -0.5, 0, +0.5, +1}.
pub fn check_wysiwyg(module: ModuleType, curve_idx: usize) -> Result<(), String> {
    let cfg = curve_display_config(module, curve_idx, GainMode::Add);
    let display_idx = display_curve_idx(module, curve_idx, GainMode::Add);

    // Use canonical attack/release for the matrix; tasks that need real
    // runtime substitution will pin specific values per index.
    let attack_ms  = 1.0;
    let release_ms = 1.0;

    for &v in &[-1.0, -0.5, 0.0, 0.5, 1.0] {
        let g_off = (cfg.offset_fn)(1.0, v);
        let display_actual = gain_to_display(
            display_idx, g_off,
            attack_ms, release_ms,
            cfg.y_min, cfg.y_max,
            /* total_history_seconds */ 0.0,
        );
        let display_expected = if v >= 0.0 {
            cfg.y_natural + v * (cfg.y_max - cfg.y_natural)
        } else {
            cfg.y_natural + v * (cfg.y_natural - cfg.y_min)
        };
        if (display_actual - display_expected).abs() > 0.5 {
            return Err(format!(
                "{:?} curve {curve_idx} (display_idx {display_idx}): \
                 v={v}: expected {display_expected:.2}, got {display_actual:.2}",
                module
            ));
        }
    }
    Ok(())
}
```

The 0.5 dB/unit tolerance is generous for floating-point noise but tight
enough to catch the kinds of bugs we're fixing (current discrepancies are
tens of dBFS).

**Helper evolution:** the version above reads anchors directly from
`cfg.y_min` / `cfg.y_max` / `cfg.y_natural`. Once §3.2's `runtime_anchors`
extension lands (substituting `global_attack_ms`/`global_release_ms` for
indices 2 and 3), the helper switches to calling `runtime_anchors(cfg,
display_idx, total_history_seconds, db_min, db_max, attack_ms, release_ms)`
and uses the returned tuple. The matrix test then pins specific
`attack_ms`/`release_ms` values per index so the check exercises the runtime
substitution path.
