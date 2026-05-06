//! Master soft clipper tests. See spec §4 of
//! 2026-05-06-stabilization-sweep.md.

use spectral_forge::params::SpectralForgeParams;
use nih_plug::prelude::Param;

#[test]
fn master_clip_enabled_default_true() {
    let p = SpectralForgeParams::default();
    assert!(p.master_clip_enabled.value(),
        "master_clip_enabled should default to true (safety-on-by-default)");
}
