//! Master soft clipper tests. See spec §4 of
//! 2026-05-06-stabilization-sweep.md.

use spectral_forge::params::SpectralForgeParams;
use spectral_forge::dsp::soft_clip::apply_soft_clip;
use nih_plug::prelude::Param;
use num_complex::Complex;

#[test]
fn master_clip_enabled_default_true() {
    let p = SpectralForgeParams::default();
    assert!(p.master_clip_enabled.value(),
        "master_clip_enabled should default to true (safety-on-by-default)");
}

#[test]
fn soft_clip_silent_input_produces_silent_output() {
    let mut bins = vec![Complex::new(0.0, 0.0); 1025];
    apply_soft_clip(&mut bins, 1025);
    for c in &bins {
        assert!(c.re.abs() < 1e-9 && c.im.abs() < 1e-9,
            "silent input should yield silent output, got {:?}", c);
    }
}

#[test]
fn soft_clip_below_threshold_attenuates_per_existing_algorithm() {
    let mut bins = vec![Complex::new(0.5, 0.0); 1025];
    apply_soft_clip(&mut bins, 1025);
    // K=4.0; at mag=0.5: scale = 4 / (4 + 0.5) = 0.889. Output ≈ 0.444.
    let expected_mag = 4.0_f32 / (4.0 + 0.5) * 0.5;
    for c in &bins {
        let got = c.norm();
        assert!((got - expected_mag).abs() < 1e-6,
            "expected mag ≈ {expected_mag}, got {got}");
    }
}

#[test]
fn soft_clip_above_threshold_no_nan_bounded() {
    let mut bins = vec![Complex::new(8.0, 0.0); 1025];
    apply_soft_clip(&mut bins, 1025);
    for c in &bins {
        assert!(c.re.is_finite() && c.im.is_finite(),
            "no NaN/Inf from soft clip");
        assert!(c.norm() < 4.5,
            "soft clip should bound magnitude near K=4, got {}", c.norm());
    }
}

#[test]
fn master_module_applies_soft_clip_when_enabled() {
    use spectral_forge::dsp::modules::master::MasterModule;
    use spectral_forge::dsp::modules::{ModuleContext, SpectralModule};
    use spectral_forge::params::{FxChannelTarget, StereoLink};

    let mut master = MasterModule::new(true);
    let mut bins = vec![Complex::new(8.0, 0.0); 1025];
    let mut supp = vec![0.0_f32; 1025];
    let ctx = ModuleContext::new(48_000.0, 2048, 1025, 10.0, 100.0, 1.0, 0.5, false, false);

    master.process(0, StereoLink::Linked, FxChannelTarget::All,
        &mut bins, None, &[], &mut supp, None, &ctx);

    // K=4 → at mag=8, scale = 4/12. Output ≈ 2.667.
    for c in &bins {
        assert!(c.norm() < 4.5, "expected clamp near K=4, got {}", c.norm());
    }
}

#[test]
fn master_module_passthrough_when_disabled() {
    use spectral_forge::dsp::modules::master::MasterModule;
    use spectral_forge::dsp::modules::{ModuleContext, SpectralModule};
    use spectral_forge::params::{FxChannelTarget, StereoLink};

    let mut master = MasterModule::new(false);
    let mut bins = vec![Complex::new(8.0, 0.0); 1025];
    let mut supp = vec![0.0_f32; 1025];
    let ctx = ModuleContext::new(48_000.0, 2048, 1025, 10.0, 100.0, 1.0, 0.5, false, false);

    master.process(0, StereoLink::Linked, FxChannelTarget::All,
        &mut bins, None, &[], &mut supp, None, &ctx);

    for c in &bins {
        assert!((c.re - 8.0).abs() < 1e-6 && c.im.abs() < 1e-6);
    }
}

#[test]
fn master_module_silent_in_silent_out_regardless_of_clip() {
    use spectral_forge::dsp::modules::master::MasterModule;
    use spectral_forge::dsp::modules::{ModuleContext, SpectralModule};
    use spectral_forge::params::{FxChannelTarget, StereoLink};

    for enabled in [true, false] {
        let mut master = MasterModule::new(enabled);
        let mut bins = vec![Complex::new(0.0, 0.0); 1025];
        let mut supp = vec![0.0_f32; 1025];
        let ctx = ModuleContext::new(48_000.0, 2048, 1025, 10.0, 100.0, 1.0, 0.5, false, false);

        master.process(0, StereoLink::Linked, FxChannelTarget::All,
            &mut bins, None, &[], &mut supp, None, &ctx);

        for c in &bins {
            assert!(c.re.abs() < 1e-9 && c.im.abs() < 1e-9,
                "silent in→silent out (enabled={enabled}); got {:?}", c);
        }
    }
}
