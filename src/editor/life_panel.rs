//! Per-slot panel widget for `LifeModule` — dev-build only.
//!
//! Renders mode-conditional MASTER_SCALE knobs for the currently selected
//! Life mode. See spec docs/superpowers/specs/2026-05-09-prototyping-exposable-scalars-design.md §1.

#![cfg(feature = "dev-build")]

use nih_plug::prelude::ParamSetter;
use nih_plug_egui::egui::{self, Ui};

use crate::dsp::modules::life::LifeMode;
use crate::editor::theme as th;
use crate::params::SpectralForgeParams;

pub fn draw(ui: &mut Ui, params: &SpectralForgeParams, setter: &ParamSetter<'_>, slot: usize) {
    if slot >= 9 { return; }
    let scale = *params.ui_scale.lock();
    let mode  = params.slot_life_mode.lock()[slot];

    ui.horizontal(|ui| {
        match mode {
            LifeMode::Viscosity      => scalar_drag(ui, scale, setter, "Viscosity ×",       params.life_viscosity_scale_param(slot)),
            LifeMode::SurfaceTension => scalar_drag(ui, scale, setter, "Surface Tension ×", params.life_surface_tension_scale_param(slot)),
            LifeMode::NonNewtonian   => scalar_drag(ui, scale, setter, "Non-Newtonian ×",   params.life_non_newtonian_scale_param(slot)),
            LifeMode::Stiction       => scalar_drag(ui, scale, setter, "Stiction ×",        params.life_stiction_scale_param(slot)),
            LifeMode::Yield          => scalar_drag(ui, scale, setter, "Yield ×",           params.life_yield_scale_param(slot)),
            LifeMode::Capillary      => scalar_drag(ui, scale, setter, "Capillary ×",       params.life_capillary_scale_param(slot)),
            LifeMode::Sandpaper      => scalar_drag(ui, scale, setter, "Sandpaper ×",       params.life_sandpaper_scale_param(slot)),
            LifeMode::Brownian       => scalar_drag(ui, scale, setter, "Brownian ×",        params.life_brownian_scale_param(slot)),
            // Crystallization and Archimedes have no clean multiplier semantics — panel renders empty.
            _ => {}
        }
    });
}

fn scalar_drag(
    ui: &mut Ui,
    scale: f32,
    setter: &ParamSetter<'_>,
    label: &str,
    param: Option<&nih_plug::prelude::FloatParam>,
) {
    if let Some(p) = param {
        ui.label(
            egui::RichText::new(label)
                .size(th::scaled(th::FONT_SIZE_LABEL, scale))
                .color(th::LABEL_DIM),
        );
        let mut v = p.value();
        let resp = ui.add(
            egui::DragValue::new(&mut v).range(0.0..=2.0).speed(0.01).fixed_decimals(2),
        );
        if resp.drag_started() { setter.begin_set_parameter(p); }
        if resp.changed()      { setter.set_parameter(p, v.clamp(0.0, 2.0)); }
        if resp.drag_stopped() { setter.end_set_parameter(p); }
    }
}
