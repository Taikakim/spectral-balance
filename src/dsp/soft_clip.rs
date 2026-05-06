//! Master output soft clipper. Originally lived in `dsp::modules::past`;
//! moved here as part of the 2026-05-06 stabilization sweep so it can run
//! at the very last output stage instead of per-PAST.
//!
//! Algorithm (unchanged from the original):
//!     scale = K / (K + |bin|)  with K = 4.0
//!     bins[k] *= scale         (only when |bin| > 1e-9 — silent → no-op)
//!
//! See docs/superpowers/specs/2026-05-06-stabilization-sweep.md §4.3.

use num_complex::Complex;

/// Soft-clip magnitudes per-bin. Silent input → silent output (the |bin| > 1e-9
/// guard ensures bit-exact passthrough at zero magnitude).
#[inline]
pub fn apply_soft_clip(bins: &mut [Complex<f32>], num_bins: usize) {
    const K: f32 = 4.0;
    for k in 0..num_bins.min(bins.len()) {
        let mag = bins[k].norm();
        if mag > 1e-9 {
            let scale = K / (K + mag);
            bins[k] *= scale;
        }
    }
}
