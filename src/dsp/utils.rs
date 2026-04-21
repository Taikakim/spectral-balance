/// xorshift64 PRNG — advances state and returns the new value.
/// State must never be zero; callers are responsible for a non-zero seed.
#[inline(always)]
pub fn xorshift64(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Convert a linear amplitude to dBFS. Returns -120.0 for values at or below the
/// noise floor threshold (1e-10 ≈ -200 dBFS), preventing log(0) and denormals.
#[inline(always)]
pub fn linear_to_db(linear: f32) -> f32 {
    if linear > 1e-10 { 20.0 * linear.log10() } else { -120.0 }
}

/// Convert a time constant in milliseconds to a one-pole IIR coefficient for an
/// envelope follower running at the STFT hop rate.
/// Returns a value in [0.0, 1.0): 0.0 = instantaneous, values near 1.0 = very slow.
#[inline(always)]
pub fn ms_to_coeff(ms: f32, sample_rate: f32, hop_size: usize) -> f32 {
    if ms < 0.001 { return 0.0; }
    let hops_per_sec = sample_rate / hop_size as f32;
    let time_hops = ms * 0.001 * hops_per_sec;
    (-1.0_f32 / time_hops).exp()
}
