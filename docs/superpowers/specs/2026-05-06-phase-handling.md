# Phase Handling — Authoritative Reference

**Status:** LIVING — read first before touching anything that operates on
spectral phase. Updated by humans with every architectural decision; AI
assistants may consult and propose updates but should not silently
override entries.

**Audience:** anyone (human or AI) editing
- `src/dsp/pipeline.rs` (PLPV unwrap / damping / rewrap)
- `src/dsp/plpv.rs` (kernels)
- `src/dsp/modules/freeze.rs` (frozen phase trajectory)
- `src/dsp/modules/phase_smear.rs` (phase randomisation)
- `src/dsp/modules/modulate.rs` (per-channel unwrap_local)
- any new module that consumes `ctx.unwrapped_phase`

**Reference repo:** `/home/kim/Projects/spectral/repos/pvx` — pvx
algorithms (Python). When in doubt about a phase-domain operation, look
there for the canonical formulation. Examples cited inline below.

---

## 1. Phase domains in this codebase

| Domain | Range | Where it lives | Notes |
|---|---|---|---|
| **Wrapped phase** | `(-π, π]` | rfft output, iFFT input | What the FFT actually produces. Use `principal_arg(x)` to canonicalise. |
| **Unwrapped phase** | unbounded conceptually, **bounded by us** to `(-π, π]` per hop | `Pipeline::prev_unwrapped_phase`, `unwrapped_phase`, `expected_phase_acc`, `freeze::frozen_unwrapped`, `modulate::unwrap_local` | Phase + integer multiples of 2π. Mathematically equivalent modulo 2π — we keep it bounded for f32 precision. |
| **Phase deviation** | small (`(-π, π]` after wrap) | inside `unwrap_phase` kernel | Difference between observed delta and expected per-hop advance. |

**Invariant (post-2026-05-06):** every accumulator that walks a phase
trajectory across hops is wrapped to `(-π, π]` after each update. This
matters for:

- `prev_unwrapped_phase[ch][k]` (pipeline)
- `unwrapped_phase[ch][k]` (pipeline) — the value modules see via `ctx.unwrapped_phase`
- `expected_phase_acc[ch][k]` (pipeline)
- `frozen_unwrapped[k]` (freeze module)
- `unwrap_local[ch][k]` (modulate module — when used as accumulator)

**If you add a new accumulator, wrap it.** Otherwise it crosses the f32
precision floor in tens of seconds at high bins / high sample rates.

---

## 2. Why we wrap (the precision wall)

The per-hop expected phase advance for bin `k` is `2π·k·hop/N`. At
fft=2048, hop=512, k=1024: advance ≈ 512π ≈ 1608 rad per hop.

f32 has 24-bit mantissa → ~7.2 decimal digits. An accumulator hits
precision loss when its absolute value exceeds ~16M (where the unit in
last place is ≥1).

At sr=96 kHz, hop=512, that's ~187 hops/sec. After
`16M / 1608 ≈ 9941` hops ≈ **53 seconds**, the high-bin accumulator
loses fractional precision. Damping then snaps low-energy bins to a
corrupted "expected" angle, audible as progressive smearing.

**Wrapping every hop** keeps the value in `(-π, π]`, avoiding the wall
indefinitely. Each hop's increment introduces a single-ULP rounding
error; those don't compound because we wrap before they accumulate.

### What does NOT work

- **Periodic reset to zero.** Tried 2026-05-06: every 4096 hops we
  zeroed `prev_unwrapped_phase` and the hop counter. This reset IS a
  phase discontinuity → spectral spreading at the reset moment →
  audible sidebands appearing at exactly the reset period. Visible in
  the bounce at `~/.BitwigStudio/...01-astral_projection_-_mantra-bounce-1.wav`
  taken on 2026-05-06 morning. **Do not reintroduce a periodic reset.**

- **Higher precision via f64.** Defers but does not solve the problem;
  modules that maintain their own f32 accumulators still drift, and
  doubling the buffer footprint costs cache.

- **Rewriting `unwrap_phase` to wrap internally.** The kernel is
  correct; the wrap belongs at the call sites where state is owned.

---

## 3. Phase blending — the wraparound trap

Linear blending of two phase angles `(1-α)·φ₁ + α·φ₂` is **wrong** at
wraparound. Example: φ₁ = π−ε, φ₂ = −π+ε, α = 0.5. The two angles are
nearly equal (both ≈ π); the linear blend gives 0 (the long way around).

### Where it's tolerable

`damp_low_energy_bins` uses linear blending in phase space. This is
acceptable because:
- it only touches bins below `noise_floor + 6 dB`
- those bins are perceptually inaudible at the iFFT stage
- the cost of the correct math is not justified at noise-floor magnitudes

### Where it must be geodesic

Anywhere a user-audible bin gets blended. Use **complex-space blending**
(pvx convention, `repos/pvx/src/pvx/core/voc.py:2505-2506`):

```rust
let blended = Complex::<f32>::from_polar(1.0, φ₁) * (1.0 - α)
            + Complex::<f32>::from_polar(1.0, φ₂) * α;
let φ_out = blended.arg();
```

This is the angle of the linear combination of unit vectors — the
geodesic on the unit circle. φ₁ = π−ε, φ₂ = −π+ε, α = 0.5 gives `φ_out
≈ π`, the true midpoint.

**Sites that use complex blending:**
- `freeze.rs` `frozen_unwrapped` ↔ live `unwrapped[k]` mix (since
  2026-05-06 — use this same shape for any new "phase A vs phase B" mix)
- pvx reference `apply_phase_engine` and `phase_vocoder_time_stretch`

**Sites that use linear blending (intentionally, low-energy only):**
- `damp_low_energy_bins` in `plpv.rs`

If you find a site doing linear blending on user-audible bins, switch
it to complex blending.

---

## 4. The unwrap kernel (`plpv::unwrap_phase`)

```
expected_advance = 2π·k·hop/N
observed_delta   = curr_phase[k] - prev_phase[k]
deviation        = principal_arg(observed_delta - expected_advance)
true_advance     = expected_advance + deviation
out_unwrapped[k] = prev_unwrapped[k] + true_advance
prev_unwrapped[..num_bins] := out_unwrapped[..num_bins]
```

Reference: Laroche–Dolson 1999, "Improved Phase Vocoder Time-Scale
Modification of Audio."

**Important:** the kernel does NOT wrap `out_unwrapped`. The pipeline
wraps after the call (since 2026-05-06). If the kernel ever moves the
wrap inside, drop the wrap at the call site.

**Edge case:** at bins where `expected_advance ≡ π (mod 2π)` exactly,
the half-open `(-π, π]` convention pulls the deviation to `+π` and the
accumulator picks up a spurious `2π` per hop. `damp_low_energy_bins`
masks this for bins below noise floor; bins above noise floor are
acoustically dominated by their own observed deviation, so the issue
is masked there too. Don't try to "fix" it without a very specific
counterexample.

---

## 5. Damping low-energy bins (`plpv::damp_low_energy_bins`)

For bins with magnitude `m`, blend factor `b`:

```
b = 1                                  if m ≤ noise_floor − 6 dB
b = smoothstep((band_hi − m) / band_w) if noise_floor − 6 dB < m < noise_floor + 6 dB
b = 0                                  if m ≥ noise_floor + 6 dB
unwrapped[k] := unwrapped[k] · (1 − b) + expected_phase[k] · b
```

The blend is linear in phase space — see §3 for why this is OK
specifically here.

`expected_phase[k]` MUST be in the same modulo-2π space as
`unwrapped[k]`. Both are wrapped to `(-π, π]` per hop by the pipeline
(since 2026-05-06).

---

## 6. Channel handling

Each channel runs through the STFT closure once per hop and maintains
its own state. State arrays indexed `[ch][k]`:

- `prev_phase`, `prev_unwrapped_phase`, `unwrapped_phase`,
  `expected_phase_acc`, `peak_buf`

Two channels grow independently at the correct per-channel rate. Do
not collapse channels into a single state; phase trajectories diverge
in real audio.

---

## 7. Module consumers of `ctx.unwrapped_phase`

| Module | Uses it as | Wraps own accumulator? |
|---|---|---|
| Freeze | reference, blends frozen vs live | Yes (since 2026-05-06) — complex blend |
| PhaseSmear | reference, adds randomised offset in-place | N/A — no own accumulator |
| Modulate | source for `unwrap_local` (or computes locally if `None`) | Yes when accumulating per-hop |

When adding a new module that consumes phase: read this section and §3
first. If you maintain an accumulator, follow the wrapping rule.

---

## 8. Checklist for new phase code

- [ ] Does it accumulate phase across hops? → wrap to `(-π, π]` after each update.
- [ ] Does it blend two phases? → complex-space (§3) for audible bins, linear-OK only at noise floor.
- [ ] Does it set `bins[k]` directly? → likely wrong — set `ctx.unwrapped_phase[k]` instead, the pipeline rewraps before iFFT.
- [ ] Did you test with `cargo test --test empty_slot_smear_soak`? → that test pins the wrap pattern.
- [ ] Did you smoke-test in Bitwig with sustained tones for ≥60 sec at sr=96 kHz with no slots loaded? → that's the regression scenario.

---

## 9. Update log

- 2026-05-06: Initial spec written. Captures the 2026-05-06 stabilization
  sweep findings: the periodic-reset misfire (rolled back), the bounded-
  accumulator + complex-blend solution, the pvx reference for blending.
  Triggered by user request: "we should have a spec on the handling of
  phase for different tasks ... documentation should be kept and I can
  then direct you to it where necessary."
