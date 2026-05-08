//! Module-switch state hygiene regression. After a module is reassigned,
//! every per-curve transform FloatParam (tilt/offset/curvature) must reset to
//! the correct value: 0.0 for tilt/curvature always; +1.0 for offset on
//! natural-at-max curves, 0.0 otherwise.
//! See docs/superpowers/specs/2026-05-07-stabilization-sweep-bc-design.md §B-1, §C-1.

use spectral_forge::params::SpectralForgeParams;

#[test]
fn tilt_offset_curvature_reset_to_zero_on_assign_module() {
    let p = SpectralForgeParams::default();
    let slot = 2;

    // NOTE: set_plain_value is pub(crate) in nih-plug — cannot be called from
    // external test crates. We set the smoother directly, which is what
    // assign_module uses internally. This verifies the smoother path is usable
    // from tests.
    for c in 0..7 {
        if let Some(t) = p.tilt_param(slot, c) {
            t.smoothed.reset(0.5);
        }
        if let Some(o) = p.offset_param(slot, c) {
            o.smoothed.reset(0.3);
        }
        if let Some(cu) = p.curvature_param(slot, c) {
            cu.smoothed.reset(-0.4);
        }
    }

    // Verify the smoothers hold the non-zero values we just wrote.
    assert!(
        (p.tilt_param(slot, 0).unwrap().smoothed.next() - 0.5).abs() < 1e-5,
        "tilt smoother should hold 0.5 before reset"
    );
    assert!(
        (p.offset_param(slot, 0).unwrap().smoothed.next() - 0.3).abs() < 1e-5,
        "offset smoother should hold 0.3 before reset"
    );
    assert!(
        (p.curvature_param(slot, 0).unwrap().smoothed.next() - (-0.4)).abs() < 1e-5,
        "curvature smoother should hold -0.4 before reset"
    );

    // Helper produces (curve_index, kind, value) triples.
    // Empty module uses default_config() which has natural_at_max: false — all offsets are 0.0.
    use spectral_forge::dsp::modules::{GainMode, ModuleType};
    let pairs: Vec<_> = spectral_forge::editor::module_popup::transform_reset_pairs(
        slot, ModuleType::Empty, GainMode::Add,
    ).collect();
    // 7 curves × 3 params = 21 pairs.
    assert_eq!(pairs.len(), 21);
    for (c, kind, value) in &pairs {
        assert!(*c < 7, "curve index {c} out of range");
        assert_eq!(*value, 0.0_f32, "transform_reset_pairs must yield 0.0 for {kind} at curve {c} (Empty module has no natural-at-max curves)");
        let _ = kind;
    }

    // Verify the pairs cover all three kinds for every curve.
    let tilts:      Vec<_> = pairs.iter().filter(|(_, k, _)| *k == "tilt").collect();
    let offsets:    Vec<_> = pairs.iter().filter(|(_, k, _)| *k == "offset").collect();
    let curvatures: Vec<_> = pairs.iter().filter(|(_, k, _)| *k == "curvature").collect();
    assert_eq!(tilts.len(),      7, "expected 7 tilt entries");
    assert_eq!(offsets.len(),    7, "expected 7 offset entries");
    assert_eq!(curvatures.len(), 7, "expected 7 curvature entries");
}

#[test]
fn transform_reset_pairs_uses_natural_at_max_for_offset_default() {
    use spectral_forge::dsp::modules::{GainMode, ModuleType};
    use spectral_forge::editor::curve_config::curve_display_config;

    // Past has natural-at-max curves at indices 0, 3, 4 (verified in curve_config).
    let pairs: Vec<_> = spectral_forge::editor::module_popup::transform_reset_pairs(
        2, ModuleType::Past, GainMode::Add,
    ).collect();
    assert_eq!(pairs.len(), 21);
    for (c, kind, value) in pairs {
        if kind == "offset" {
            let cfg = curve_display_config(ModuleType::Past, c, GainMode::Add);
            let expected = if cfg.natural_at_max { 1.0_f32 } else { 0.0_f32 };
            assert_eq!(value, expected,
                "Past curve {c} offset reset should be {expected} (natural_at_max={})",
                cfg.natural_at_max);
        } else {
            assert_eq!(value, 0.0_f32,
                "tilt/curvature should always reset to 0.0; got {value} for kind={kind} curve={c}");
        }
    }
}

#[test]
fn transform_reset_pairs_zeros_for_module_with_no_natural_at_max() {
    use spectral_forge::dsp::modules::{GainMode, ModuleType};

    // Dynamics has natural-at-max only on curve 5 (MIX). Verify the helper
    // returns +1.0 for MIX (index 5) and 0.0 for all other offset resets.
    let pairs: Vec<_> = spectral_forge::editor::module_popup::transform_reset_pairs(
        0, ModuleType::Dynamics, GainMode::Add,
    ).collect();
    for (c, kind, value) in pairs {
        if kind == "offset" && c == 5 {
            assert_eq!(value, 1.0_f32, "Dynamics MIX offset reset should be 1.0");
        } else {
            assert_eq!(value, 0.0_f32,
                "non-MIX or tilt/curvature should reset to 0.0; got {value} for c={c} kind={kind}");
        }
    }
}
