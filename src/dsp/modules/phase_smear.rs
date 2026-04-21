use num_complex::Complex;
use crate::params::{FxChannelTarget, StereoLink};
use crate::dsp::utils::xorshift64;
use crate::dsp::pipeline::{MAX_NUM_BINS, OVERLAP};
use super::{ModuleContext, ModuleType, SpectralModule};

pub struct PhaseSmearModule {
    rng_state: u64,
    peak_env: Vec<f32>,
    sample_rate: f32,
    fft_size: usize,
}

impl PhaseSmearModule {
    pub fn new() -> Self {
        Self {
            rng_state: 0x123456789abcdef0,
            peak_env: vec![0.0f32; MAX_NUM_BINS],
            sample_rate: 44100.0,
            fft_size: 2048,
        }
    }

    #[inline]
    fn curve_to_hold_ms(curve: f32) -> f32 {
        let c = curve.clamp(0.0, 2.0);
        let log_min = 1.0f32.ln();
        let log_mid = 50.0f32.ln();
        let log_max = 500.0f32.ln();
        let log_t = if c <= 1.0 {
            log_min + (log_mid - log_min) * c
        } else {
            log_mid + (log_max - log_mid) * (c - 1.0)
        };
        log_t.exp()
    }
}

impl Default for PhaseSmearModule {
    fn default() -> Self { Self::new() }
}

impl SpectralModule for PhaseSmearModule {
    fn reset(&mut self, sample_rate: f32, fft_size: usize) {
        self.sample_rate = sample_rate;
        self.fft_size = fft_size;
        for v in &mut self.peak_env { *v = 0.0; }
    }

    fn process(
        &mut self,
        _channel: usize,
        _stereo_link: StereoLink,
        _target: FxChannelTarget,
        bins: &mut [Complex<f32>],
        sidechain: Option<&[f32]>,
        curves: &[&[f32]],
        suppression_out: &mut [f32],
        _ctx: &ModuleContext,
    ) {
        if bins.is_empty() { suppression_out.fill(0.0); return; }
        let last = bins.len() - 1;
        let hop_ms = self.fft_size as f32 / (OVERLAP as f32 * self.sample_rate) * 1000.0;

        for k in 0..bins.len() {
            let dry = bins[k];
            // Always advance PRNG to keep the sequence independent of skipping.
            let rand = xorshift64(&mut self.rng_state);
            // DC (k=0) and Nyquist (k=last) must stay real for IFFT correctness.
            if k == 0 || k == last { continue; }

            let sc_raw = sidechain.and_then(|s| s.get(k)).copied().unwrap_or(0.0).max(0.0);
            let hold_c = curves.get(1).and_then(|c| c.get(k)).copied().unwrap_or(1.0);
            let hold_ms = Self::curve_to_hold_ms(hold_c);
            let rel = (-hop_ms / hold_ms.max(0.1)).exp();
            if sc_raw > self.peak_env[k] {
                self.peak_env[k] = sc_raw;
            } else {
                self.peak_env[k] = rel * self.peak_env[k] + (1.0 - rel) * sc_raw;
            }
            let sc_mod = self.peak_env[k].min(1.0);

            let amount_curve = curves.get(0).and_then(|c| c.get(k))
                               .copied().unwrap_or(1.0).clamp(0.0, 2.0);
            let per_bin = (amount_curve * (1.0 + sc_mod)).clamp(0.0, 2.0);

            let scale      = per_bin * std::f32::consts::PI;
            let rand_phase = (rand as f32 / u64::MAX as f32 * 2.0 - 1.0) * scale;
            let (mag, phase) = (bins[k].norm(), bins[k].arg());
            let wet = Complex::from_polar(mag, phase + rand_phase);
            let mix = curves.get(2).and_then(|c| c.get(k)).copied().unwrap_or(1.0).clamp(0.0, 1.0);
            bins[k] = Complex::new(
                dry.re * (1.0 - mix) + wet.re * mix,
                dry.im * (1.0 - mix) + wet.im * mix,
            );
        }
        suppression_out.fill(0.0);
    }

    fn module_type(&self) -> ModuleType { ModuleType::PhaseSmear }
    fn num_curves(&self) -> usize { 3 }
}
