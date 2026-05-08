//! Centralized parameter ID formatting. Single source of truth for both
//! build.rs code generation and runtime param lookup.
//!
//! IDs are STABLE FOREVER — changing any formatting here will break
//! saved automation lanes in user projects.

pub const NUM_SLOTS: usize = 9;
pub const NUM_CURVES: usize = 7;
pub const NUM_NODES: usize = 6;

/// Number of rows in the automation-exposed matrix grid. Rows are
/// DESTINATIONS in the routing-matrix semantic (param `mr{dst}c{src}`),
/// so this stays at 9 (only slots can receive sends; virtual rows are
/// sources, not destinations).
pub const NUM_MATRIX_ROWS: usize = 9;

/// Number of source columns in the automation-exposed matrix grid (param
/// `mr{dst}c{src}`). 9 real slots + 4 T/S Split virtual rows
/// (transient + sustained for up to 2 active T/S Splits, matching
/// `dsp::modules::MAX_SPLIT_VIRTUAL_ROWS`). Aligned with
/// `dsp::modules::MAX_MATRIX_ROWS` so virtual-row send levels are
/// persisted and host-automatable (2026-05-08).
pub const NUM_MATRIX_SOURCES: usize = 13;

pub fn graph_node_id(slot: usize, curve: usize, node: usize, field: char) -> String {
    debug_assert!(matches!(field, 'x' | 'y' | 'q'));
    format!("s{}c{}n{}{}", slot, curve, node, field)
}

pub fn tilt_id(slot: usize, curve: usize) -> String {
    format!("s{}c{}tilt", slot, curve)
}

pub fn offset_id(slot: usize, curve: usize) -> String {
    format!("s{}c{}offset", slot, curve)
}

pub fn curvature_id(slot: usize, curve: usize) -> String {
    format!("s{}c{}curv", slot, curve)
}

pub fn matrix_id(row: usize, col: usize) -> String {
    format!("mr{}c{}", row, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_node_id_format() {
        assert_eq!(graph_node_id(0, 0, 0, 'x'), "s0c0n0x");
        assert_eq!(graph_node_id(8, 6, 5, 'q'), "s8c6n5q");
    }

    #[test]
    fn tilt_offset_matrix_ids() {
        assert_eq!(tilt_id(2, 3), "s2c3tilt");
        assert_eq!(offset_id(2, 3), "s2c3offset");
        assert_eq!(matrix_id(1, 4), "mr1c4");
    }

    #[test]
    fn curvature_id_format() {
        assert_eq!(curvature_id(4, 3), "s4c3curv");
        assert_eq!(curvature_id(0, 0), "s0c0curv");
    }

    #[test]
    fn total_counts() {
        assert_eq!(NUM_SLOTS * NUM_CURVES * NUM_NODES * 3, 1134);
        assert_eq!(NUM_SLOTS * NUM_CURVES * 2, 126);  // tilt + offset
        assert_eq!(NUM_SLOTS * NUM_CURVES * 3, 189);  // tilt + offset + curvature
        // 9 dest rows × 13 source cols = 117 (9 real slot sources + 4 virtual).
        assert_eq!(NUM_MATRIX_ROWS * NUM_MATRIX_SOURCES, 117);
    }
}
