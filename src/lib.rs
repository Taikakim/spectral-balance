pub mod dsp;
pub mod editor;
pub mod params;
pub mod bridge;

use nih_plug::prelude::*;
use params::SpectralForgeParams;
use std::sync::Arc;

pub struct SpectralForge {
    params:   Arc<SpectralForgeParams>,
    pipeline: Option<dsp::pipeline::Pipeline>,
    shared:   Option<bridge::SharedState>,
}

impl Default for SpectralForge {
    fn default() -> Self {
        Self {
            params:   Arc::new(SpectralForgeParams::default()),
            pipeline: None,
            shared:   None,
        }
    }
}

impl Plugin for SpectralForge {
    const NAME: &'static str = "Spectral Forge";
    const VENDOR: &'static str = "Kim";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> { self.params.clone() }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let sr = buffer_config.sample_rate;
        let num_ch = audio_io_layout.main_output_channels
            .map(|c| c.get() as usize).unwrap_or(2);
        let num_bins = dsp::pipeline::FFT_SIZE / 2 + 1;
        self.shared   = Some(bridge::SharedState::new(num_bins, sr));
        self.pipeline = Some(dsp::pipeline::Pipeline::new(sr, num_ch));
        context.set_latency_samples(dsp::pipeline::FFT_SIZE as u32);
        true
    }

    fn reset(&mut self) {}

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        dsp::guard::flush_denormals();
        if let (Some(pipeline), Some(shared)) = (&mut self.pipeline, &mut self.shared) {
            pipeline.process(buffer, shared);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for SpectralForge {
    const CLAP_ID: &'static str = "com.spectral-forge.spectral-forge";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Spectral compressor");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect, ClapFeature::Stereo,
    ];
}

nih_export_clap!(SpectralForge);
