# Phase 1 — Stabilization Sweep Diagnostics

## 1. Routing matrix break

**Bug confirmed:** GUI→params→pipeline chain breaks at the GUI→params link.

**Evidence:**
- `paint_fx_matrix_grid` at `src/editor/fx_matrix_grid.rs:211-228` mutates `route_matrix.send[row][col]` via a raw `&mut f32` reference exposed to the egui DragValue.
- The audio path reads from `params.matrix_cell(r, col).smoothed.next()` at `src/dsp/pipeline.rs:970`. This is a separate FloatParam.
- The Arc<Mutex<RouteMatrix>> in params is read by pipeline ONLY for `virtual_rows`, `amp_mode`, `amp_params` (lines 953-959) — NOT for `send`.
- Convention is correct: `route_matrix.send[src][dst] ↔ matrix_cell(dst, src)` per the comment at `params.rs:793`.

**Fix shape:** rewire the DragValue at `fx_matrix_grid.rs:217` to call
`setter.set_parameter(matrix_cell(col, row), value)` instead of mutating
the f32 directly. Caller (editor_ui.rs) must pass a `&ParamSetter` into
`paint_fx_matrix_grid`.

## 2. Smearing-over-time accumulator

**Audit findings:**

State containers checked (file:lines where updated):

| Container | File:lines | Clears per block? | Gated? | Stateful across blocks? |
|---|---|---|---|---|
| `slot_out[s]` | `fx_matrix.rs:578,597,651` | Yes (overwritten every hop for every slot) | N/A | No |
| `slot_supp[s]` | `fx_matrix.rs:579,598` | Yes (filled to 0 every hop) | N/A | No |
| `mix_buf` | `fx_matrix.rs:510,675` | Yes (zeroed at top of each slot iteration and Master) | N/A | No |
| `amp_scratch` | `fx_matrix.rs:518,533,680` | Yes (overwritten before each use) | N/A | No |
| `amp_state[ch][r][c]` | `fx_matrix.rs:521,536,683` | No — IIR state (Vactrol/Slew/Stiction/Schmitt) persists across hops | Skipped when `send < 0.001`; default mode is Linear (no state) | Yes, for non-Linear cells only |
| `prev_mags` | `fx_matrix.rs:562` | No — explicitly carries across hops for auto-velocity | Yes — inside `if self.bin_physics_in_use` | Yes, but gated |
| `slot_phys`, `mix_phys` | `fx_matrix.rs:554,574,647` | No | Yes — inside `if self.bin_physics_in_use` | Yes, but gated |
| `sc_env_states` | `pipeline.rs:617-619` | No — IIR state decays when signal is absent | Yes — zeroed when `!has_aux` (no sidechain input) | Yes, but bounded by IIR decay |
| `history` (HistoryBuffer) | `pipeline.rs:1263` | No — ring buffer, advances every hop unconditionally | Not gated on any module need | Yes — but write-only from outside modules; no audible output path when all slots Empty |
| `if_prev_phase` | `pipeline.rs:1162` | No — carries analysis-frame phase for IF computation | Yes — inside `if any_needs_if` | Yes, but gated |
| `prev_phase[ch]` | `pipeline.rs:1094` | No — carries previous hop's wrapped phase | Yes — inside `if plpv_enable` | Yes, but gated |
| `prev_unwrapped_phase[ch]` | `plpv.rs:69` (called from pipeline.rs:1084) | No — accumulates true-advance sum across hops | Yes — inside `if plpv_enable` | **Yes — grows without bound as f32** |
| `total_hops_per_ch[ch]` | `pipeline.rs:1116` | No — monotonically increasing hop counter | Yes — inside `if plpv_enable` | **Yes — grows without bound** |
| `ring_transforms` | `pipeline.rs:539` | No — RingTransformState persists across blocks | Yes — skipped when `ring_snapshot.entry_count() == 0` | Yes, but gated |

**`FxMatrix::clear_state()` gap (secondary finding):**
`Pipeline::clear_state()` (the GUI Reset path) calls `fx_matrix.clear_state()` at `pipeline.rs:331`,
which zeroes `slot_out`, `slot_supp`, `virtual_out`, `mix_buf`, and amp state —
but does NOT reset `prev_mags`, `slot_phys`, or `mix_phys`. These are only zeroed in
`fx_matrix.reset()`, called from `Pipeline::reset()` (init + FFT-size change). With
`bin_physics_in_use = false` (all slots Empty), these fields are never read, so this gap is
currently inert. If a physics module is ever loaded-then-removed, stale physics values survive
GUI Reset but are still gated out. Noted for Task 12 (reset audit).

**Identified primary accumulator:**
`prev_unwrapped_phase[ch]` (`src/dsp/pipeline.rs:1087`, via `plpv.rs:69`) combined with
`total_hops_per_ch` (`pipeline.rs:1116`).

Both update unconditionally whenever `plpv_enable == true`, which defaults to `true`
(`params.rs:577`). `prev_unwrapped_phase[ch][k]` accumulates the sum of per-hop true-phase
advances for every bin: after N hops it equals approximately `two_pi_hop_over_n * k * N`.
For bin k=1024 at FFT=2048, OVERLAP=4, 44100 Hz sample rate, after ~30 minutes of
operation this value reaches ≈ 2.78 × 10^8 radians. f32 has only ≈ 16 radians of
fractional precision at that magnitude, so the stored phase loses all sub-radian resolution.

`damp_low_energy_bins` (`plpv.rs:89-120`, called at `pipeline.rs:1109`) blends
low-energy bins toward `scratch_expected_ref[k] = two_pi_hop_over_n * k * hop_total`
(pipeline.rs:1104). This `scratch_expected` value suffers the same f32 overflow — it is
recomputed from the growing `hop_total` every hop. The blend target therefore contains
large quantization errors for high-frequency bins, causing those bins to be "damped"
toward a wrong phase that varies with quantization artefacts.

`rewrap_phase` (`plpv.rs:73-79`, called at `pipeline.rs:1224`) then reduces the corrupted
`unwrapped_phase` back to (-π, π] and reconstructs `complex_buf` from (magnitude, wrapped
phase). Even though magnitude is preserved, the corrupted phase produces audibly different
phase relationships across overlapping STFT frames, which the Hann-squared OLA adds
constructively in MAGNITUDE but destructively in PHASE — causing progressive smearing of
transients and spectral detail. The effect is worst for the highest-frequency bins (largest k)
and worsens as `hop_total` grows. Power-cycling the plugin calls `Pipeline::new()` which
resets both `total_hops_per_ch = [0; 2]` and `prev_unwrapped_phase.fill(0.0)`, clearing
the accumulated error.

This accumulation occurs even with ALL slots Empty because `plpv_enable` is not gated on
whether any module requests `ctx.unwrapped_phase`. The PLPV machinery runs unconditionally
when the flag is true.

**Proposed fix shape:**

Two complementary options (Task 11 picks one):

Option A — **Periodic phase reset**: every `M` hops (e.g. M = 4096 ≈ 30 s at 44100/2048/4),
reset `prev_unwrapped_phase[ch].fill(0.0)` and `total_hops_per_ch[ch] = 0`. Accept a
one-hop discontinuity; the damping blend is a soft function so the audible click is minimal.
This is the lowest-risk change. Choose M so that `two_pi_hop_over_n * k_max * M` stays
below ~1 × 10^6 (where f32 still has ≤ 1 radian of quantization error).

Option B — **Phase residual only**: instead of accumulating absolute unwrapped phase,
store only the fractional residual relative to the expected advance each hop. Concretely:
after `unwrap_phase`, subtract `scratch_expected_ref[k]` from `unwrapped_phase_ref[ch][k]`
before passing to `damp_low_energy_bins`, then add it back for the rewrap. This keeps the
stored values bounded to O(π) regardless of session length, eliminating the precision
problem entirely. More invasive but more correct.

Option C — **Gate PLPV on active module demand**: only run the PLPV machinery when at
least one active slot's `module_spec` declares `needs_plpv` (a new flag). With all Empty
slots this would be a no-op. Does not fix the drift for long sessions with PLPV-using
modules loaded, but eliminates the "no modules" symptom completely.

**Default recommendation for Task 11:** Option A (periodic reset at M=4096 hops) because
it is minimally invasive and directly addresses the observed symptom. If the user reports
residual drift with PLPV modules loaded, follow with Option B.

**If no clear accumulator found via static audit:** the implementer of
Task 11 falls back to the probe-instrumented harness `tests/empty_slot_smear_audit.rs`
and runs it with `cargo test --features=probe --test empty_slot_smear_audit -- --include-ignored --nocapture`. The probe outputs identify the accumulator empirically.

**Backup plan if accumulator is intrinsic to STFT (per spec §5.8):**
default to option β (periodic forced-reset every 1024 blocks). Task 11
documents the choice in the §5.8 update of the tracker doc and proceeds
with the implementation. The user can override on review.
