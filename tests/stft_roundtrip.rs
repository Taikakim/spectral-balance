// tests/stft_roundtrip.rs
// Verifies that identity processing through the STFT → ISTFT pipeline
// preserves a sine wave with max error < 1e-3.

use approx::assert_abs_diff_eq;

#[test]
fn sine_roundtrip_identity() {
    use spectral_forge::dsp::pipeline::process_block_for_test;

    let sample_rate = 44100.0f32;
    let freq = 440.0f32;
    let n_samples = 8192usize;

    let input: Vec<f32> = (0..n_samples)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
        .collect();

    let output = process_block_for_test(&input, sample_rate);

    // Skip first FFT_SIZE samples (pipeline latency)
    let latency = spectral_forge::dsp::pipeline::FFT_SIZE;
    for i in latency..n_samples {
        assert_abs_diff_eq!(
            output[i], input[i - latency],
            epsilon = 1e-3,
        );
    }
}
