use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use parking_lot::Mutex;
use std::sync::Arc;
use crate::editor::curve::CurveNode;

pub const NUM_CURVE_SETS: usize = 7;
pub const NUM_NODES: usize = 6;

/// Index into the 7 parameter curve sets.
pub mod curve_idx {
    pub const THRESHOLD: usize = 0;
    pub const RATIO:     usize = 1;
    pub const ATTACK:    usize = 2;
    pub const RELEASE:   usize = 3;
    pub const KNEE:      usize = 4;
    pub const MAKEUP:    usize = 5;
    pub const MIX:       usize = 6;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum ThresholdMode { Absolute, Relative }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum StereoLink { Independent, Linked, MidSide }

#[derive(Params)]
pub struct SpectralForgeParams {
    #[persist = "editor_state"]
    pub editor_state: Arc<EguiState>,

    #[persist = "curve_nodes"]
    pub curve_nodes: Arc<Mutex<[[CurveNode; NUM_NODES]; NUM_CURVE_SETS]>>,

    #[persist = "active_curve"]
    pub active_curve: Arc<Mutex<usize>>,

    #[id = "input_gain"]
    pub input_gain: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "attack_ms"]
    pub attack_ms: FloatParam,

    #[id = "release_ms"]
    pub release_ms: FloatParam,

    #[id = "freq_scale"]
    pub freq_scale: FloatParam,

    #[id = "sc_gain"]
    pub sc_gain: FloatParam,

    #[id = "sc_attack_ms"]
    pub sc_attack_ms: FloatParam,

    #[id = "sc_release_ms"]
    pub sc_release_ms: FloatParam,

    #[id = "lookahead_ms"]
    pub lookahead_ms: FloatParam,

    #[id = "stereo_link"]
    pub stereo_link: EnumParam<StereoLink>,

    #[id = "threshold_mode"]
    pub threshold_mode: EnumParam<ThresholdMode>,

    #[id = "auto_makeup"]
    pub auto_makeup: BoolParam,

    #[id = "delta_monitor"]
    pub delta_monitor: BoolParam,
}

impl SpectralForgeParams {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            editor_state: EguiState::from_size(900, 600),
            curve_nodes: Arc::new(Mutex::new(
                [crate::editor::curve::default_nodes(); NUM_CURVE_SETS]
            )),
            active_curve: Arc::new(Mutex::new(0)),

            input_gain: FloatParam::new(
                "Input Gain", 0.0,
                FloatRange::Linear { min: -18.0, max: 18.0 },
            ).with_smoother(SmoothingStyle::Logarithmic(20.0))
             .with_unit(" dB"),

            output_gain: FloatParam::new(
                "Output Gain", 0.0,
                FloatRange::Linear { min: -18.0, max: 18.0 },
            ).with_smoother(SmoothingStyle::Logarithmic(20.0))
             .with_unit(" dB"),

            mix: FloatParam::new(
                "Mix", 1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ).with_smoother(SmoothingStyle::Linear(10.0)),

            attack_ms: FloatParam::new(
                "Attack", 10.0,
                FloatRange::Skewed { min: 0.5, max: 200.0, factor: FloatRange::skew_factor(-2.0) },
            ).with_smoother(SmoothingStyle::Logarithmic(20.0))
             .with_unit(" ms"),

            release_ms: FloatParam::new(
                "Release", 80.0,
                FloatRange::Skewed { min: 1.0, max: 500.0, factor: FloatRange::skew_factor(-2.0) },
            ).with_smoother(SmoothingStyle::Logarithmic(20.0))
             .with_unit(" ms"),

            freq_scale: FloatParam::new(
                "Freq Scale", 0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ).with_smoother(SmoothingStyle::Linear(50.0)),

            sc_gain: FloatParam::new(
                "SC Gain", 0.0,
                FloatRange::Linear { min: -18.0, max: 18.0 },
            ).with_smoother(SmoothingStyle::Logarithmic(20.0))
             .with_unit(" dB"),

            sc_attack_ms: FloatParam::new(
                "SC Attack", 5.0,
                FloatRange::Skewed { min: 0.5, max: 100.0, factor: FloatRange::skew_factor(-2.0) },
            ).with_smoother(SmoothingStyle::Logarithmic(20.0))
             .with_unit(" ms"),

            sc_release_ms: FloatParam::new(
                "SC Release", 50.0,
                FloatRange::Skewed { min: 1.0, max: 300.0, factor: FloatRange::skew_factor(-2.0) },
            ).with_smoother(SmoothingStyle::Logarithmic(20.0))
             .with_unit(" ms"),

            lookahead_ms: FloatParam::new(
                "Lookahead", 0.0,
                FloatRange::Linear { min: 0.0, max: 10.0 },
            ).with_unit(" ms"),

            stereo_link: EnumParam::new("Stereo Link", StereoLink::Linked),
            threshold_mode: EnumParam::new("Threshold Mode", ThresholdMode::Absolute),
            auto_makeup: BoolParam::new("Auto Makeup", false),
            delta_monitor: BoolParam::new("Delta Monitor", false),
        })
    }
}
