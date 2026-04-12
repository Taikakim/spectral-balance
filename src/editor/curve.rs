use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CurveNode {
    pub x: f32,  // [0.0, 1.0] normalised log-frequency
    pub y: f32,  // [-1.0, +1.0] gain: 0.0 = neutral
    pub q: f32,  // [0.0, 1.0] normalised octave-bandwidth
}

pub fn default_nodes() -> [CurveNode; 6] {
    [
        CurveNode { x: 0.0,  y: 0.0, q: 0.3 },
        CurveNode { x: 0.2,  y: 0.0, q: 0.5 },
        CurveNode { x: 0.4,  y: 0.0, q: 0.5 },
        CurveNode { x: 0.6,  y: 0.0, q: 0.5 },
        CurveNode { x: 0.8,  y: 0.0, q: 0.5 },
        CurveNode { x: 1.0,  y: 0.0, q: 0.3 },
    ]
}

#[derive(Clone, Copy, Debug)]
pub enum BandType { LowShelf, Bell, HighShelf }

pub fn band_type_for(index: usize) -> BandType {
    match index {
        0 => BandType::LowShelf,
        5 => BandType::HighShelf,
        _ => BandType::Bell,
    }
}

/// Convert normalised node fields to physical units.
fn node_to_physical(node: &CurveNode) -> (f32, f32, f32) {
    let freq_hz = 20.0 * 1000.0f32.powf(node.x);   // 20 Hz – 20 kHz log-scaled
    let gain_db = node.y * 18.0;                      // ±18 dB
    let bw_oct  = 0.1 * 40.0f32.powf(node.q);        // 0.1 – 4.0 octaves
    (freq_hz, gain_db, bw_oct)
}

/// Smooth bell curve magnitude response centered at f0.
/// Uses a Gaussian-like shape in log-frequency space for numerical stability.
fn magnitude_bell_curve(f_hz: f32, f0: f32, gain_db: f32, bw_oct: f32) -> f32 {
    if gain_db.abs() < 1e-6 { return 1.0; }

    // Gaussian width in log-frequency: sigma = bw_oct / 2.355 (4-sigma = bandwidth)
    let sigma = bw_oct / 2.355;
    let log_ratio = (f_hz / f0).abs().max(0.001).ln() / std::f32::consts::LN_2;  // log2 frequency ratio
    let exponent = -(log_ratio * log_ratio) / (2.0 * sigma * sigma);
    let bell = exponent.exp();

    let gain_linear = 10.0f32.powf(gain_db / 20.0);  // Linear gain factor
    1.0 + (gain_linear - 1.0) * bell
}

/// Smooth shelf response: transitions from 1.0 to gain over the bandwidth.
fn magnitude_shelf_curve(f_hz: f32, f0: f32, gain_db: f32, bw_oct: f32, is_high: bool) -> f32 {
    if gain_db.abs() < 1e-6 { return 1.0; }

    let gain_linear = 10.0f32.powf(gain_db / 20.0);
    let log_ratio = (f_hz / f0).max(0.001).ln() / std::f32::consts::LN_2;  // log2 frequency ratio

    // Transition width: ±2 octaves from the center
    let transition_width = 2.0 + bw_oct;
    let t = if is_high {
        (log_ratio + transition_width / 2.0) / transition_width  // High shelf
    } else {
        (-log_ratio + transition_width / 2.0) / transition_width  // Low shelf
    };

    let s = t.clamp(0.0, 1.0);
    // Smooth step using cubic Hermite: s_smooth = 3s² - 2s³
    let s_smooth = 3.0*s*s - 2.0*s*s*s;

    1.0 + (gain_linear - 1.0) * s_smooth
}

/// Compute magnitude response for a single EQ band.
/// Note: uses Gaussian/Hermite log-frequency approximations, not time-domain IIR biquad.
/// This is intentional — the curve feeds a frequency-domain gain array, not a sample-rate filter.
fn eq_band_magnitude(f_hz: f32, f0: f32, gain_db: f32, bw_oct: f32,
                     band: BandType) -> f32 {
    match band {
        BandType::Bell => magnitude_bell_curve(f_hz, f0, gain_db, bw_oct),
        BandType::LowShelf => magnitude_shelf_curve(f_hz, f0, gain_db, bw_oct, false),
        BandType::HighShelf => magnitude_shelf_curve(f_hz, f0, gain_db, bw_oct, true),
    }
}

/// Compute combined magnitude response for all 6 nodes at num_bins frequencies.
/// Returns a Vec<f32> of linear gain values (1.0 = unity, >1 = boost, <1 = cut).
pub fn compute_curve_response(
    nodes: &[CurveNode; 6],
    num_bins: usize,
    sample_rate: f32,
    fft_size: usize,
) -> Vec<f32> {
    let mut gains = vec![1.0f32; num_bins];

    for (i, node) in nodes.iter().enumerate() {
        if node.y.abs() < 1e-4 { continue; }
        let (freq_hz, gain_db, bw_oct) = node_to_physical(node);
        let band = band_type_for(i);

        for k in 0..num_bins {
            let f_bin = (k as f32 * sample_rate / fft_size as f32).max(1.0);
            let mag = eq_band_magnitude(f_bin, freq_hz, gain_db, bw_oct, band);
            gains[k] *= mag;
        }
    }

    for g in &mut gains { *g = g.max(0.0); }
    gains
}
