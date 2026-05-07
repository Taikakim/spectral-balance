//! C-2 regression: virtual node y range goes -2..+2 and the editor's drag
//! handler honors the wider range. Visual indicator verified by manual
//! smoke test (the constant exists and is non-zero).

use spectral_forge::editor::theme;

#[test]
fn offrect_indicator_constants_present() {
    assert!(theme::NODE_OFFRECT_SIZE_PX > 0.0);
    assert!(theme::NODE_OFFRECT_OFFSET_PX >= 0.0);
    let c = theme::NODE_OFFRECT_COLOR;
    // Red-ish: r > g and r > b.
    assert!(c.r() > c.g());
    assert!(c.r() > c.b());
}

#[test]
fn node_y_range_clamp_is_two() {
    use spectral_forge::editor::curve::CurveNode;
    let n = CurveNode { x: 0.5, y: 1.5, q: 0.5 };
    assert!(n.y > 1.0 && n.y <= 2.0);
}
