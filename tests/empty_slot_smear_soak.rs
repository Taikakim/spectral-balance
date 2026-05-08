//! Smearing-over-time regression. The PLPV phase accumulator
//! (prev_unwrapped_phase + an expected-phase counter) used to grow
//! unboundedly, causing progressive smearing on the wet path even with
//! no modules loaded. The original fix was a periodic reset every 4096
//! hops; that introduced an audible phase discontinuity (sidebands at
//! the reset moment — see screenshot in 2026-05-06 morning testing).
//!
//! Replacement fix: wrap both `prev_unwrapped_phase` and the
//! `expected_phase_acc` accumulator to `(-π, π]` after every hop. Phase
//! is musically defined modulo 2π, so wrapping is a no-op for downstream
//! consumers (re-wrap before iFFT, modulo-space comparisons in modules).
//! The accumulator never crosses the f32 precision floor and there is no
//! discontinuity → no sidebands.
//!
//! See docs/superpowers/specs/2026-05-06-stabilization-sweep.md §5.

use spectral_forge::dsp::plpv::principal_arg;
use std::f32::consts::PI;

#[test]
fn principal_arg_keeps_repeated_increments_bounded() {
    // Simulate the per-hop accumulator update pattern:
    //   acc = principal_arg(acc + 2π·k·hop/N)
    // Over many hops, `acc` must stay in (-π, π] regardless of how
    // large the per-hop increment is.
    let two_pi_hop_over_n = 2.0 * PI * 512.0 / 2048.0; // π/2 (fft=2048, hop=512)
    for k in (0..1024).step_by(64) {
        let increment = two_pi_hop_over_n * k as f32;
        let mut acc: f32 = 0.0;
        // 1 million hops ≈ 1.5 hours at fft=2048/sr=96k.
        for _ in 0..1_000_000 {
            acc = principal_arg(acc + increment);
            assert!(acc > -PI - 1e-3 && acc <= PI + 1e-3,
                "k={k}: accumulator escaped (-π, π], got {acc}");
            assert!(acc.is_finite(), "k={k}: accumulator went non-finite, got {acc}");
        }
    }
}

#[test]
fn principal_arg_increment_modulo_consistency() {
    // The damping step blends `unwrapped[k]` toward
    // `expected_phase[k]`. Both must be in the same modulo-2π space for
    // the linear blend to be musically stable. This test verifies that
    // a known angle (e.g., 1.234 rad) is preserved by the
    // increment+wrap pattern after many cycles.
    //
    // Strategy: increment by a value that is an exact multiple of 2π;
    // after the wrap, the accumulator should equal the starting value
    // (mod numerical tolerance from f32 rounding of the increment).
    let mut acc: f32 = 1.234;
    let increment = 2.0 * PI * 100.0; // 100 full cycles per step
    for _ in 0..10_000 {
        acc = principal_arg(acc + increment);
    }
    // After many "exact 2π·n" increments, acc should still equal 1.234
    // up to f32 rounding (a few ULPs accumulated over 10k iterations).
    assert!((acc - 1.234).abs() < 1e-2,
        "modulo-2π identity broke: expected ~1.234, got {acc}");
}
