use approx::assert_abs_diff_eq;

#[test]
fn flat_curve_unity_gains() {
    use spectral_forge::editor::curve::{default_nodes, compute_curve_response};
    let nodes = default_nodes(); // all y=0.0 → neutral → linear gain = 1.0
    let gains = compute_curve_response(&nodes, 1025, 44100.0, 2048);
    assert_eq!(gains.len(), 1025);
    for &g in gains.iter() {
        assert_abs_diff_eq!(g, 1.0, epsilon = 1e-4);
    }
}

#[test]
fn full_boost_greater_than_unity() {
    use spectral_forge::editor::curve::compute_curve_response;
    let mut nodes = spectral_forge::editor::curve::default_nodes();
    for n in &mut nodes { n.y = 1.0; }
    let gains = compute_curve_response(&nodes, 1025, 44100.0, 2048);
    for &g in &gains {
        assert!(g > 1.0, "boost should be > 1.0, got {}", g);
    }
}

#[test]
fn full_cut_less_than_unity() {
    use spectral_forge::editor::curve::compute_curve_response;
    let mut nodes = spectral_forge::editor::curve::default_nodes();
    for n in &mut nodes { n.y = -1.0; }
    let gains = compute_curve_response(&nodes, 1025, 44100.0, 2048);
    for &g in &gains {
        assert!(g < 1.0, "cut should be < 1.0, got {}", g);
        assert!(g >= 0.0, "gain must be non-negative");
    }
}
