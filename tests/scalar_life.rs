//! Life scalars: default-correctness + plumbing.
use spectral_forge::dsp::modules::life::LifeScalars;

#[test]
fn life_safe_default_matches_hardcoded_values() {
    let s = LifeScalars::safe_default();
    assert_eq!(s.viscosity_scale, 1.0);
    assert_eq!(s.surface_tension_scale, 1.0);
    assert_eq!(s.non_newtonian_scale, 1.0);
    assert_eq!(s.stiction_scale, 1.0);
    assert_eq!(s.yield_scale, 1.0);
    assert_eq!(s.capillary_scale, 1.0);
    assert_eq!(s.sandpaper_scale, 1.0);
    assert_eq!(s.brownian_scale, 1.0);
}

#[test]
#[cfg(feature = "probe")]
fn life_scalars_round_trip_through_fx_matrix() {
    use spectral_forge::dsp::fx_matrix::FxMatrix;
    use spectral_forge::dsp::modules::ModuleType;

    // Slot 0 = Life; everything else Empty.
    let slot_types: [ModuleType; 9] = [
        ModuleType::Life, ModuleType::Empty, ModuleType::Empty,
        ModuleType::Empty, ModuleType::Empty, ModuleType::Empty,
        ModuleType::Empty, ModuleType::Empty, ModuleType::Empty,
    ];
    let mut fxm = FxMatrix::new(48_000.0, 2048, &slot_types);

    let custom = LifeScalars {
        viscosity_scale: 0.25,
        ..LifeScalars::safe_default()
    };
    let mut arr = [LifeScalars::safe_default(); 9];
    arr[0] = custom;

    fxm.set_life_scalars(&arr);
    let read_back = fxm.test_life_scalars(0).expect("slot 0 should hold Life");
    assert!((read_back.viscosity_scale - 0.25).abs() < 1e-6);
}
