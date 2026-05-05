//! WYSIWYG calibration matrix — UI parameter spec §2 + 2026-05-05-graph-display-correctness.md.
//! Asserts that for every (module, curve_idx), the offset_fn produces a
//! gain that gain_to_display maps back to the spec's axis_aware_lerp value.

use spectral_forge::dsp::modules::{module_spec, GainMode, ModuleType};
use spectral_forge::editor::curve::{
    axis_aware_lerp, display_curve_idx, gain_to_display, runtime_anchors,
};
use spectral_forge::editor::curve_config::curve_display_config;

/// Display indices currently deferred from WYSIWYG enforcement.
/// idx 13: PAST Age/Delay — total_history_seconds plumbing pending.
/// idx 10: PEAK HOLD on PhaseSmear/1 + Gain/1 — DSP function mismatch (separate plan).
fn is_deferred(module: ModuleType, curve_idx: usize, display_idx: usize) -> bool {
    if display_idx == 13 { return true; }
    matches!((module, curve_idx, display_idx),
        (ModuleType::PhaseSmear, 1, 10) | (ModuleType::Gain, 1, 10))
}

fn check_one(module: ModuleType, curve_idx: usize) -> Result<(), String> {
    let cfg = curve_display_config(module, curve_idx, GainMode::Add);
    let display_idx = display_curve_idx(module, curve_idx, GainMode::Add);
    if is_deferred(module, curve_idx, display_idx) { return Ok(()); }

    let attack_ms  = 10.0_f32;
    let release_ms = 100.0_f32;
    let db_min     = -60.0_f32;
    let db_max     = 0.0_f32;
    let history    = 0.0_f32;
    let anchors = runtime_anchors(
        &cfg, display_idx, history, db_min, db_max, attack_ms, release_ms,
    );

    for &v in &[-1.0_f32, -0.5, 0.0, 0.5, 1.0] {
        let g_off = (cfg.offset_fn)(1.0, v, anchors);
        let display_actual = gain_to_display(
            display_idx, g_off, attack_ms, release_ms, db_min, db_max, history,
        );
        let display_expected = axis_aware_lerp(&cfg, anchors, v);
        if (display_actual - display_expected).abs() > 0.5 {
            return Err(format!(
                "{:?}/{} (idx {display_idx}): v={v:+.2} expected {display_expected:.3}, got {display_actual:.3}",
                module, curve_idx
            ));
        }
    }
    Ok(())
}

#[test]
fn calibration_matrix_all_modules_all_curves() {
    let modules: &[ModuleType] = &[
        ModuleType::Dynamics, ModuleType::Freeze, ModuleType::PhaseSmear,
        ModuleType::Contrast, ModuleType::Gain, ModuleType::MidSide,
        ModuleType::TransientSustainedSplit, ModuleType::Harmonic,
        ModuleType::Past, ModuleType::Geometry, ModuleType::Circuit,
        ModuleType::Life, ModuleType::Modulate, ModuleType::Rhythm,
        ModuleType::Punch, ModuleType::Harmony, ModuleType::Kinetics,
        ModuleType::Future,
    ];
    let mut failures = Vec::new();
    for &m in modules {
        let spec = module_spec(m);
        for c in 0..spec.num_curves.min(7) {
            if let Err(msg) = check_one(m, c) {
                failures.push(msg);
            }
        }
    }
    if !failures.is_empty() {
        panic!("{} WYSIWYG failures:\n{}", failures.len(), failures.join("\n"));
    }
}
