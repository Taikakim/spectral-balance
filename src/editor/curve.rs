use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CurveNode {
    pub x: f32,  // [0.0, 1.0] normalised log-frequency
    pub y: f32,  // [-1.0, +1.0] normalised gain/effect
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
