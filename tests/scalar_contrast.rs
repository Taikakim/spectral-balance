//! Contrast THRESHOLD bypass + mode + scalars regression suite.
use spectral_forge::dsp::engines::spectral_contrast::SpectralContrastEngine;
use spectral_forge::dsp::engines::{BinParams, SpectralEngine};
use num_complex::Complex;

#[test]
fn contrast_threshold_bins_below_floor_bypass() {
    let mut engine = SpectralContrastEngine::new();
    engine.reset(48_000.0, 1024);
    let n = 513;

    // bin[0..256] sit at -60 dBFS, bin[256..] sit at 0 dBFS.
    // THRESHOLD set to -40 dBFS: bins below it should bypass, above should be processed.
    let mut bins: Vec<Complex<f32>> = (0..n)
        .map(|k| if k < 256 { Complex::new(1e-3, 0.0) } else { Complex::new(1.0, 0.0) })
        .collect();
    let original: Vec<Complex<f32>> = bins.clone();
    let threshold: Vec<f32> = vec![-40.0; n];
    let ratio:     Vec<f32> = vec![5.0; n];
    let attack:    Vec<f32> = vec![10.0; n];
    let release:   Vec<f32> = vec![100.0; n];
    let knee:      Vec<f32> = vec![0.0; n];
    let makeup:    Vec<f32> = vec![0.0; n];
    let mix:       Vec<f32> = vec![1.0; n];
    let mut suppression: Vec<f32> = vec![0.0; n];

    let params = BinParams {
        threshold_db: &threshold, ratio: &ratio, attack_ms: &attack, release_ms: &release,
        knee_db: &knee, makeup_db: &makeup, mix: &mix, smoothing_semitones: 1.0,
        sensitivity: 1.0, auto_makeup: false,
        peaks: None, plpv_dynamics_enabled: false,
    };
    engine.process_bins(&mut bins, None, &params, 48_000.0, &mut suppression);

    // Bins below threshold (k < 256) must be untouched.
    for k in 0..256 {
        assert!((bins[k].re - original[k].re).abs() < 1e-6,
            "bin {k}: expected unchanged (below threshold), got {:?}", bins[k]);
    }
}
