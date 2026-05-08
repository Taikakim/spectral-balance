use nih_plug_egui::egui::{self, Pos2, Ui};
use crate::dsp::amp_modes::AmpMode;
use crate::dsp::modules::{MAX_SLOTS, MAX_MATRIX_ROWS};
use crate::editor::theme as th;
use crate::params::SpectralForgeParams;

/// Ephemeral state for the amp-cell popup. Stored in egui temp data.
#[derive(Clone)]
pub struct AmpPopupState {
    pub open: bool,
    pub row:  usize,
    pub col:  usize,
    pub pos:  Pos2,
}

impl Default for AmpPopupState {
    fn default() -> Self {
        Self { open: false, row: 0, col: 0, pos: Pos2::ZERO }
    }
}

const MODES: &[AmpMode] = &[
    AmpMode::Linear, AmpMode::Vactrol, AmpMode::Schmitt, AmpMode::Slew, AmpMode::Stiction,
];

/// Render the popup if open. Call every frame from the main UI closure.
/// Returns true if the popup consumed a click.
pub fn show_popup(ui: &mut Ui, params: &SpectralForgeParams, scale: f32) -> bool {
    let key = ui.id().with("amp_popup");
    let state: AmpPopupState = ui.data(|d| d.get_temp(key).unwrap_or_default());
    if !state.open { return false; }

    let (row, col) = (state.row, state.col);
    if row >= MAX_MATRIX_ROWS || col >= MAX_SLOTS {
        ui.data_mut(|d| d.insert_temp(key, AmpPopupState::default()));
        return false;
    }

    let (mut current_mode, current_amount, current_threshold, current_release, current_slew) = {
        let rm = params.route_matrix.lock();
        (
            rm.amp_mode[row][col],
            rm.amp_params[row][col].amount,
            rm.amp_params[row][col].threshold,
            rm.amp_params[row][col].release_ms,
            rm.amp_params[row][col].slew_db_per_s,
        )
    };

    let mut new_state = state.clone();
    let mut consumed = false;
    let mut mode_changed = false;
    let mut amount = current_amount;
    let mut threshold = current_threshold;
    let mut release  = current_release;
    let mut slew     = current_slew;

    egui::Area::new(egui::Id::new("amp_popup_area"))
        .fixed_pos(state.pos)
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);
                ui.label(
                    egui::RichText::new(format!("Amp ({}, {})", row, col))
                        .color(th::LABEL_DIM).size(th::scaled(th::FONT_SIZE_LABEL, scale))
                );
                ui.separator();

                for &mode in MODES {
                    let selected = current_mode == mode;
                    let resp = ui.selectable_label(selected, mode.label());
                    crate::editor::help_box::track_help_strings(
                        ui, &resp, mode.label(), mode.hint(),
                    );
                    if resp.clicked() && !selected {
                        current_mode = mode;
                        mode_changed = true;
                        consumed = true;
                    }
                }

                ui.separator();
                let amount_resp = ui.add(egui::Slider::new(&mut amount, 0.0..=2.0).text("amount"));
                crate::editor::help_box::track_help_strings(
                    ui, &amount_resp,
                    "Amp · amount",
                    "Strength of the selected amp filter. 0 = bypass (the send acts as if the filter were Linear), 1 = full effect, >1 exaggerates. The same value drives every filter type, so dialling this in lets you A/B-compare types at matched intensity.",
                );
                let thr_resp = ui.add(egui::Slider::new(&mut threshold, 0.0..=1.0).text("threshold"));
                crate::editor::help_box::track_help_strings(
                    ui, &thr_resp,
                    "Amp · threshold",
                    "Trip level (linear magnitude 0..1). Schmitt: bin must exceed this to latch on; the lower trip point sits below it via internal hysteresis. Stiction: minimum step size before the send budges. Ignored by Linear / Vactrol / Slew.",
                );
                let rel_resp = ui.add(egui::Slider::new(&mut release, 1.0..=2000.0).text("release ms"));
                crate::editor::help_box::track_help_strings(
                    ui, &rel_resp,
                    "Amp · release ms",
                    "Vactrol release time. The send envelope decays toward the input at this rate; longer values give the classic opto-coupler tail. Ignored by Linear / Schmitt / Slew / Stiction.",
                );
                let slew_resp = ui.add(egui::Slider::new(&mut slew, 1.0..=240.0).text("slew dB/s"));
                crate::editor::help_box::track_help_strings(
                    ui, &slew_resp,
                    "Amp · slew dB/s",
                    "Maximum rate of change for the send level (Slew filter). Lower = slower glide between targets, smearing fast modulation into a smooth ramp. Ignored by every other filter type.",
                );

                ui.separator();
                if ui.button("Close").clicked() {
                    new_state.open = false;
                    consumed = true;
                }
            });
        });

    let needs_write = mode_changed
        || (amount    - current_amount).abs()    > 1e-6
        || (threshold - current_threshold).abs() > 1e-6
        || (release   - current_release).abs()   > 1e-6
        || (slew      - current_slew).abs()      > 1e-6;
    if needs_write {
        let mut rm = params.route_matrix.lock();
        rm.amp_mode[row][col] = current_mode;
        rm.amp_params[row][col].amount        = amount;
        rm.amp_params[row][col].threshold     = threshold;
        rm.amp_params[row][col].release_ms    = release;
        rm.amp_params[row][col].slew_db_per_s = slew;
    }

    ui.data_mut(|d| d.insert_temp(key, new_state));
    consumed
}

/// Open the popup at `pos` for cell (row, col). Call from a click handler.
pub fn open_at(ui: &mut Ui, row: usize, col: usize, pos: Pos2) {
    let key = ui.id().with("amp_popup");
    ui.data_mut(|d| d.insert_temp(key, AmpPopupState { open: true, row, col, pos }));
}
