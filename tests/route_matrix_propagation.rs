//! Phase 1 regression: pinning the GUI→DSP routing break observed in 2026-05-06
//! diagnostics. After Phase 2, this test asserts the chain works.

use spectral_forge::params::SpectralForgeParams;

#[test]
fn matrix_cell_param_exists_for_all_valid_coordinates() {
    use spectral_forge::param_ids::{NUM_MATRIX_ROWS, NUM_SLOTS};
    let p = SpectralForgeParams::default();
    for r in 0..NUM_MATRIX_ROWS {
        for col in 0..NUM_SLOTS {
            if r == col { continue; } // diagonal is module-loaded indicator, not a route
            assert!(p.matrix_cell(r, col).is_some(),
                "matrix_cell({r}, {col}) should be Some");
        }
    }
    // Out-of-range returns None.
    assert!(p.matrix_cell(NUM_MATRIX_ROWS, 0).is_none());
    assert!(p.matrix_cell(0, NUM_SLOTS).is_none());
}
