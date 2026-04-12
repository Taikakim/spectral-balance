// THE ONLY file that defines visual constants. Reskin by editing this file.

use nih_plug_egui::egui::Color32;

// Backgrounds
pub const BG:         Color32 = Color32::from_rgb(0x12, 0x12, 0x14);
pub const GRID:       Color32 = Color32::from_rgb(0x1a, 0x2a, 0x28);

// Structural lines
pub const BORDER:     Color32 = Color32::from_rgb(0x00, 0xcc, 0xbb);
pub const DIVIDER:    Color32 = Color32::from_rgb(0x00, 0x88, 0x80);

// Curve
pub const CURVE:      Color32 = Color32::from_rgb(0x00, 0xff, 0xdd);
pub const NODE_FILL:  Color32 = Color32::from_rgb(0x00, 0xcc, 0xbb);
pub const NODE_HOVER: Color32 = Color32::from_rgb(0x44, 0xff, 0xee);

// Text
pub const LABEL:      Color32 = Color32::from_rgb(0x88, 0xdd, 0xcc);
pub const LABEL_DIM:  Color32 = Color32::from_rgb(0x44, 0x88, 0x80);

// Buttons
pub const BTN_ACTIVE:   Color32 = Color32::from_rgb(0x00, 0xcc, 0xbb);
pub const BTN_INACTIVE: Color32 = Color32::from_rgb(0x22, 0x33, 0x30);
pub const BTN_TEXT_ON:  Color32 = Color32::from_rgb(0x00, 0x10, 0x0e);
pub const BTN_TEXT_OFF: Color32 = Color32::from_rgb(0x88, 0xdd, 0xcc);

// Stroke widths
pub const STROKE_THIN:   f32 = 1.0;
pub const STROKE_BORDER: f32 = 1.5;
pub const STROKE_CURVE:  f32 = 1.5;
pub const NODE_RADIUS:   f32 = 5.0;

/// Spectrum / suppression bar colour. Input: normalised magnitude [0.0, 1.0].
/// Gradient: dark blue → blue → green → yellow → red.
pub fn magnitude_color(norm: f32) -> Color32 {
    let n = norm.clamp(0.0, 1.0);
    if n < 0.25 {
        let t = n / 0.25;
        Color32::from_rgb(
            0,
            (20.0 * t) as u8,
            (80.0 + 120.0 * t) as u8,
        )
    } else if n < 0.5 {
        let t = (n - 0.25) / 0.25;
        Color32::from_rgb(
            0,
            (20.0 + 180.0 * t) as u8,
            (200.0 - 150.0 * t) as u8,
        )
    } else if n < 0.75 {
        let t = (n - 0.5) / 0.25;
        Color32::from_rgb(
            (200.0 * t) as u8,
            200,
            (50.0 - 50.0 * t) as u8,
        )
    } else {
        let t = (n - 0.75) / 0.25;
        Color32::from_rgb(
            (200.0 + 55.0 * t) as u8,
            (200.0 - 200.0 * t) as u8,
            0,
        )
    }
}
