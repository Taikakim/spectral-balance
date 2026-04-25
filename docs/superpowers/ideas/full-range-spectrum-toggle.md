# Idea: Full-range vs. audible-range display toggle

## Problem
Display always extends to host Nyquist. At 192 kHz that puts the audible range
(20 Hz – 20 kHz) into the leftmost ~70 % of the rect, and the upper grid lines
in `HZ_VERTICALS_HI` only go to 45 kHz so 45 k – 96 k looks empty. Conversely,
when bin-mirroring or other future high-frequency processing matters, the full
range *should* be visible.

## Proposed UX
A small arrow / chevron toggle in the lower-right of the curve area, two states:

- **Audible** (default): clamp visible range to 20 Hz – 20 kHz regardless of
  sample rate.
- **Full**: visible range = 20 Hz – Nyquist (current behaviour).

Switch only affects the *display* range — DSP is untouched, all bins still
processed.

## Touchpoints
- New persisted GUI param (`show_full_range: bool` in `params.rs`).
- New small widget in the curve area (probably painted, not a real egui widget,
  to fit the lower-right corner cleanly).
- `freq_to_x_max`, `paint_grid`, `paint_response_curve`,
  `paint_spectrum_and_suppression`, `paint_peak_hold_envelope_overlay`,
  `screen_to_freq`: all currently take Nyquist as `max_hz`. Replace with a
  helper `fn display_max_hz(sr, full_range_flag)` and call it once per frame.
- Extend `HZ_VERTICALS_HI` to cover 45 k – 96 k (or generate dynamically).

## Why deferred
Touches param schema, persistence, layout, and 5 painter functions. Needs
brainstorming + a small spec update before implementation. Not appropriate as
an in-line patch to the calibration audit.
