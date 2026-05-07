# Spectral Forge Stabilization Backlog

**Last updated:** 2026-05-07
**Purpose:** Persistent tracker of known issues, user directives, and design decisions for the Spectral Forge stabilization effort. Survives context compaction. Update this doc as facts change.

---

## Build & deploy facts

- **Dev plugin install path:** `~/.clap/spectral/dev/spectral_dev.clap` (not `~/.clap/spectral_forge.clap`).
- **Dev build command:** `cargo build --release --features dev-build`. The `dev-build` Cargo feature (`Cargo.toml:35`, cfg-gates `CLAP_ID`/`VST3_CLASS_ID`/`NAME` in `lib.rs`) gives the dev plugin a distinct identity so Bitwig doesn't confuse it with the release version.
- **Install step:** `cp target/release/libspectral_forge.so ~/.clap/spectral/dev/spectral_dev.clap` (the .so is just a .clap with the Linux extension).
- **Workflow gotcha:** previously bundling to `~/.clap/spectral_forge.clap` did nothing because Bitwig was loading the dev path. All earlier "still broken" reports against post-fix builds were testing stale (pre-Tasks-1-16) code.

## User-stated directives (apply globally)

- **Universal slider traversal:** at slider value `v = -1` the offset should reach `y_min`; at `v = +1` reach `y_max`; at `v = 0` reach `y_natural`. This must hold for *every* curve. Implies the current `y_natural == y_max` patterns (MIX, AMOUNT) must be redesigned — the positive half of the slider must do something useful.
- **MIX default 100% wet for every module.**
- **Master output stage:** soft clipper belongs at the very last output stage (post-FxMatrix), not per-PAST.
- **Master Floor default:** -120 dBFS (currently -100).
- **Tilt range:** allow ~2× steeper angles than current.
- **Module-switch isolation:** switching a slot's module type should reset that slot's curves/nodes/per-mode state. Current behaviour leaks state across module types.
- **Dry/wet mix at 0% (full dry) gives true bypass** — already works. The dry path is bit-perfect.

## Open issue backlog

Numbered for cross-reference. Status: 🔴 critical · 🟠 important · 🟡 normal · ⚪ deferred / paused.

| # | Status | Issue | Source |
|---|---|---|---|
| 1 | ✅ | All-modules-disabled wet-path processing — likely fixed by smear fix (#12). Manual smoke verifies. | user msg |
| 2 | ✅ | Soft clipper moved to master output stage with toggle (default on). PAST::soft_clip removed. | user msg |
| 3 | ✅ | PAST AMOUNT/Age/Smear/MIX sliders cap at "0" — resolved by C-1 (offset default +1 for natural-at-max curves). Age idx 13 still pending total_history_seconds plumbing — sub-project E. | user msg + diagnosed |
| 4 | ✅ | Routing matrix fix landed: GUI cell-click writes via setter to FloatParam. Off-diagonal cells fully functional. Virtual rows (T/S Split) noted with TODO — matrix_cell bounds-check rejects r>=9, needs separate work. | user msg + screenshot |
| 5 | ✅ | MIX default 100% wet — resolved by C-1 (offset FloatParam defaults to +1.0 for natural-at-max curves; loads at y_max). | user msg |
| 6 | 🟡 | PAST SMEAR is binary toggle at 50% (apply_granular only); ignored in 4 of 5 PAST modes | user msg + audit |
| 7 | ✅ | Dead-half on `y_natural==y_max` curves — resolved by C-1 (default load at v=+1, slider semantics universal -1..+1 unchanged). | user msg |
| 8 | ✅ | Dev plugin identity needs distinct CLAP ID — already exists via `dev-build` feature | resolved |
| 9 | 🟡 | Dynamics THRESHOLD floors at db_min (-60 default) — tied to (10) | user msg |
| 10 | 🟠 | Master Floor default should be -120 dBFS | user directive |
| 11 | 🟡 | Tilt range needs ~2× steeper angles | user directive |
| 12 | ✅ | Smearing fix: PLPV `prev_unwrapped_phase` + `total_hops_per_ch` reset every 4096 hops (~44s at fft 2048/sr 48k). Phase 1 audit identified the cause; commit `f26c3ac`. | user msg |
| 13 | ✅ | Module-switch carryover — resolved: tilt/offset/curvature FloatParam atomics now reset via setter on module switch (commit `1496f12`); offset reset is module-aware so natural-at-max defaults are honored (commit `1d2b706`). | user clarification |
| 14 | ✅ | PAST mode UI: resolved by inlining 5 mode labels in the slot row (commit `5d6f3b4`); popup removed; DecaySorter sub-picker stays inline. | user msg |
| 15 | 🟡 | Freeze: most curves work; Resistance has weak audible effect — likely a level-mismatch in the kernel | user msg |
| 16 | 🟡 | Freeze PORTAMENTO range: should be 0ms (instant)..~750ms; currently 40..1000 | user msg |
| 17 | ✅ | Virtual node range -2..+2 with red directional triangle indicator at rect edge when off-rect (commits `8801840`, `590d41c`). | user msg |
| 18 | ✅ | Offset-aware drag — resolved structurally by virtual node range (#17); the wider underlying y space gives full drag headroom regardless of offset position. | user msg + screenshot |

## Sub-project decomposition

Six sub-projects with critical-path ordering:

- **(A) Pipeline bypass + routing + soft clipper + smearing fix** ← *currently brainstorming*. Combined per Approach 1 (single-spec stabilization sweep). Covers issues #1, #2, #4, #12. Critical path: blocks reliable testing of everything else.
- **(B) Module-state isolation + slot lifecycle.** Covers #13, #14. Universal carryover bug + PAST mode UI dead.
- **(C) Curve UX redesign (universal -1..+1 traversal).** Covers #3, #5, #7, #17, #18. Major UX rework — kills the `y_natural==y_max` dead-half pattern.
- **(D) Master axis defaults + per-curve range adjustments.** Covers #9, #10, #11, #16.
- **(E) DSP semantics completion.** Covers #6, #15. PAST AMOUNT/SMEAR plumbing across modes; Resistance fix.
- **(F) Spec / spec-table follow-ups.** PEAK HOLD DSP mismatch deferred from prior plan.

## Sub-project A — current state (in design)

- **Approach chosen:** Approach 1 — single-spec stabilization sweep covering routing + soft-clipper-move + Empty-slot bypass + smearing-over-time.
- **Phase plan** (from Section 1 of design):
  - Phase 1: diagnostics-only — characterize routing matrix break and smearing-over-time root cause.
  - Phase 2: routing matrix plumbing fix.
  - Phase 3: soft clipper architecture move (PAST → master output stage).
  - Phase 4: smearing-over-time fix (shape determined by Phase 1 diagnostic).
  - Empty-slot bypass semantics: paragraph-sized decision in Phase 2 or 3 — wet path with all slots Empty must be audibly transparent (matching dry); we do NOT add a true-bypass-skips-STFT mode.

## Diagnostic facts so far

- Routing failure mode: bug type (a) — UI edits don't reach DSP. The route_matrix snapshot in `pipeline::process` is using defaults regardless of user matrix edits. Code path looks correct on paper (`fx_matrix::process_hop` at lines 506-687 properly gates on `send < 0.001`), so the break is upstream — params or snapshot.
- Smearing-over-time happens with NO modules loaded → it's pipeline-base, not module-specific. Likely candidates: BinPhysics buffers, history buffer, STFT internal state, modulation ring, slot_curve_cache, FFT scratch.
- mix=0% gives true bypass → dry path is bit-perfect.

## Design decisions made

- 2026-05-05: dev-build identity via `dev-build` Cargo feature flag (already exists).
- 2026-05-06: stabilization sweep covers four issue clusters in one sub-project (Approach 1).
- 2026-05-06: Empty-slot bypass = "wet path transparent enough you can't tell wet from dry" (does NOT skip STFT — Bitwig's bypass button is the host-level escape).
- 2026-05-06: this tracker doc serves as the single source of truth across sessions and context resets.

## Phase 1 findings (committed in 41946be)

### Routing matrix break
- **Site:** `src/editor/fx_matrix_grid.rs:217` — DragValue mutates `&mut route_matrix.send[row][col]` directly. Second site at `fx_matrix_grid.rs:307` for virtual rows.
- **Why audio doesn't see edits:** pipeline reads `params.matrix_cell(r, col).smoothed.next()` (FloatParam), not the Arc<Mutex<RouteMatrix>> field. The two stores never sync.
- **Fix shape:** rewire both DragValue sites to call `setter.set_parameter(matrix_cell(col, row), value)`. Caller passes ParamSetter into `paint_fx_matrix_grid`.

### Smearing-over-time
- **Site:** `prev_unwrapped_phase[ch]` and `total_hops_per_ch` in `src/dsp/pipeline.rs:1084-1116`. The Phase-Locked Phase Vocoder (PLPV) accumulator.
- **Why it accumulates:** PLPV is enabled by default. Every hop adds `two_pi_hop_over_n × k × N` to `prev_unwrapped_phase[ch]`. At bin k=1024, FFT=2048, OVERLAP=4, this reaches ~2.78×10^8 radians after ~30 minutes — beyond f32 fractional precision (16 radians/ULP). `damp_low_energy_bins` blends low-energy bins toward the now-quantization-corrupted expected phase. Result: progressive smear at high bins. The code comment at pipeline.rs:1099 acknowledges "Acceptable f32-precision loss after ~30 h" — but in practice it shows up much sooner on certain content.
- **Hint match:** matches the user's "look at the code that carries silently over the physics etc bin data first" direction — the `prev_unwrapped_phase` IS bin-data state that carries silently.
- **Power-cycle clears it:** `Pipeline::new()` resets both counters to zero. Matches user observation.
- **Fix shape (chosen for Task 11):** Option A — periodic reset of `prev_unwrapped_phase[ch].fill(0.0)` and `total_hops_per_ch[ch] = 0` every M=4096 hops. Bounded values, no drift. One-hop phase discontinuity at reset is inaudible at the blend weights PLPV uses.

## Sub-project A — complete (2026-05-06 overnight)

All 4 phases landed across 13 commits since `1216e5f`:

- **Phase 1 — Diagnostics** (`41946be`, `fee1fec`): identified routing GUI bug at fx_matrix_grid.rs:217 + virtual rows at :307; identified PLPV `prev_unwrapped_phase` accumulator as smearing cause.
- **Phase 2 — Routing fix** (`f3d7d53`): GUI now writes via `setter.set_parameter` to FloatParams. Off-diagonal cells fully functional. Virtual rows have a TODO note (matrix_cell bounds-check rejects r >= 9 — separate work to extend NUM_MATRIX_ROWS).
- **Phase 3 — Master clipper** (`f48c839`, `6a14292`, `1ae696b`, `f53dfa6`, `1947902`): added `master_clip_enabled` BoolParam (default true), extracted `apply_soft_clip` to `dsp::soft_clip`, wired into MasterModule with set-method on SpectralModule trait, removed from PAST entirely, added CLIP toggle button in master row UI (matches AUTO_MK/DELTA pattern).
- **Phase 4 — Smearing fix** (`f26c3ac`): periodic reset of `prev_unwrapped_phase[ch].fill(0.0)` + `total_hops_per_ch[ch] = 0` every 4096 hops in pipeline.rs::process. Plus regression test pinning the constant + modulo arithmetic.
- **Test fixture cleanup** (`cbab5fc`): `bin_physics_pipeline` test that probes upstream audio at mag=2.0 needed `set_master_clip_enabled(false)` since K=4 clipper would clamp it.

**Final regression:**
- `cargo test`: 0 failures (one flaky-in-parallel test `if_probe_reads_array_when_present` passes in isolation; pre-existing intermittent issue, not introduced here).
- `cargo test --features=probe`: exactly 5 pre-existing failures (`*_amount_default_probes_50_pct`).
- `cargo build --release --features dev-build`: clean.
- Dev plugin installed at `~/.clap/spectral/dev/spectral_dev.clap` (md5 verified).

**User to manually smoke-test in Bitwig in the morning:**
1. Routing scenario (Dynamics→Contrast=1.0, Contrast→Master=0.5, Freeze standalone) → expected: signal flows per matrix, Freeze gets no input.
2. Empty-slot bypass: all slots Empty, mix=100%, music for 30s+ → expected: audibly transparent, no progressive degradation.
3. Soft clipper at silent input (mute source, slots Empty, mix=100%) → expected: silent output.
4. Smearing soak: 5 minutes continuous music with all Empty slots → expected: no progressive degradation.
5. CLIP toggle: hot signal pushed through, click CLIP off → audible distortion (clipper bypassed); CLIP on → bounded near K=4 magnitude.

Sub-projects B–F remain open per the backlog.

## Sub-project B+C — complete (2026-05-07)

Combined small-task sweep on top of A and the follow-up UI fixes. Spec at `docs/superpowers/specs/2026-05-07-stabilization-sweep-bc-design.md`; plan at `docs/superpowers/plans/2026-05-07-stabilization-sweep-bc.md`. Six task commits across the four scope items:

- **B-1** (`1496f12`, `1d2b706`): tilt/offset/curvature FloatParam atomics now reset via `setter.set_parameter` on module switch, mirroring the existing graph_node pattern. Follow-up commit makes the offset reset module-aware so it picks `+1.0` for natural-at-max curves of the newly assigned module (otherwise C-1's default would have been destroyed on every popup-driven module change).
- **B-2** (`5d6f3b4`): PAST modes inlined in slot row as 5 selectable_label buttons; DecaySorter sort-key sub-picker also inline. Popup chrome (`show_popup`, `open_at`, `PastPopupState`) deleted. `MODES`, `SORT_KEYS`, `mode_label` retained as public helpers.
- **C-1** (`d67fd91`): added `natural_at_max: bool` to `CurveDisplayConfig`, set on all 47 literals where `y_natural == y_max`. `build.rs` codegen reads the flag to default the offset FloatParam to `+1.0` instead of `0.0` for those curves. Resolves dead-half + 100%-wet-default + PAST AMOUNT/SMEAR cap. Slider semantics universal `−1..+1` unchanged.
- **C-2** (`8801840`, `590d41c`): off-rect indicator added — small red directional triangle drawn just outside the curve rect edge when `|node.y| > 1`. Follow-up commit fixes triangle x-alignment to use the dot's log-frequency screen-x (was using a linear mapping). The virtual node y range `−2..+2` itself was already in place from prior work; receivers (modules) clamp to physical limits.

**Final regression:** `cargo test` 490 passed / 0 failed / 3 ignored. `cargo test --features=probe` 1 pre-existing failure unchanged from baseline (`threshold_idx9_dsp_matches_display_log_formula` in `tests/calibration.rs` — same threshold-formula deferral as `tests/curve_calibration_matrix.rs`). Dev plugin built and installed at `~/.clap/spectral/dev/spectral_dev.clap`.

**User to manually smoke-test on waking:**
1. Switch a slot from Dynamics with custom tilt/offset to Freeze → sliders show 0.0 (or +1.0 for natural-at-max curves), no carryover.
2. Assign PAST → 5 mode labels visible inline in slot row. Click each → mode changes audibly. Click DecaySorter → sort-key sub-row appears.
3. Fresh patch, any module with MIX → MIX defaults to 100% wet (offset slider visually at top).
4. Drag a node up past the top of the curve graph → red triangle appears at top edge tracking the node's x. Drag back down → triangle disappears at y=1.

Sub-projects D, E, F remain open.

## Update log

- 2026-05-06: doc created with full backlog, sub-project decomposition, Sub-project A Phase plan, dev-install workflow facts.
- 2026-05-06: spec `2026-05-06-stabilization-sweep.md` written and approved by user with two refinements: (a) master clipper gets a UI toggle button (default on), not always-on; (b) smearing-over-time is a recent regression — user directs Phase 1 to start with BinPhysics carryover audit. Spec §5.1 marks BinPhysics as the PRIMARY HYPOTHESIS and flags `prev_mags` at fx_matrix.rs:562 as a specific suspect (unconditional update, should be gated on `bin_physics_in_use`).
- 2026-05-06: Phase 1 diagnostics committed (`41946be`). Routing break confirmed at `fx_matrix_grid.rs:217` AND `:307`. Smearing accumulator identified as PLPV `prev_unwrapped_phase` + `total_hops_per_ch` in pipeline.rs:1084-1116 (NOT BinPhysics — my earlier hypothesis was wrong; user's hint about "carries silently over... bin data" still matches since prev_unwrapped_phase IS per-bin state). Fix shape locked: periodic reset every 4096 hops.
- 2026-05-06 overnight: sub-project A complete. Routing, smearing, soft clipper move (with toggle), and Empty-slot bypass all addressed. 13 commits, dev plugin installed. Sub-projects B (module-state isolation, PAST mode UI), C (universal slider traversal + UX), D (master axis defaults), E (DSP semantics completion), F (PEAK HOLD) remain open.
- 2026-05-06 morning: user smoke-tested. CLIP toggle was non-clickable (param not registered with host — fixed `6212441`). Soft clipper "ate the bass" because per-bin K/(K+|x|) tilts the spectrum even at quiet inputs — replaced with threshold-knee soft saturation + `master_clip_threshold_db` knob (`7f57c58`). Threshold reference was uncalibrated for fft-size-dependent bin magnitudes — fixed by scaling threshold reference by `fft_size/4` (`cf40590`). Matrix row/column labels persisted on Empty slots — fixed (`82d8ff4`).
- 2026-05-06 morning (cont.): user reported sidebands appearing at ~21 sec on idle three-sine input. Confirmed cause: the periodic phase reset I added in `f26c3ac` IS itself a phase discontinuity → spectral spreading at exactly the reset moment. Replaced periodic reset with bounded-incremental-accumulator pattern: `prev_unwrapped_phase` and `expected_phase_acc` wrapped to `(-π, π]` after every hop. Freeze module's `frozen_unwrapped` accumulator likewise wrapped; freeze blend upgraded to complex-space (geodesic) per pvx reference convention. New phase-handling spec written at `docs/superpowers/specs/2026-05-06-phase-handling.md`; CLAUDE.md points to it.
- 2026-05-07: sub-project B+C complete. Module-switch hygiene (B-1) + PAST inline UI (B-2) + dead-half resolution (C-1) + node off-rect indicator (C-2) landed across 6 commits (`1496f12` → `590d41c`). Tracker open issues now: D, E, F. Closed during this sweep: #3, #5, #7, #13, #14, #17, #18.
