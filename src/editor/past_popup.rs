use crate::dsp::modules::past::{PastMode, SortKey};

pub const MODES: &[(PastMode, &str, &str)] = &[
    (PastMode::Granular,    "Granular Window",
     "Granular Window — selective time-windowed freeze of stable bins. AMOUNT picks which bins read history vs live; TIME (Age) sets per-bin lookback; THRESHOLD gates by current magnitude; SPREAD (Smear) toggles a 3-bin frequency smear of the historical read. Use it for tape-stop / freeze-with-modulation effects on long sustained material. Sidechain: not used."),
    (PastMode::DecaySorter, "Decay Sorter",
     "Decay Sorter — temporal reconstruction via summary-stat sorting. Bins are reordered along the spectrum by a chosen sort key (decay time / IF stability / RMS). Pairs with the Sort sub-picker to choose the key. Use it for a pseudo-spectral-resynth that pushes long-ringing partials toward where the loud peaks were. Sidechain: not used."),
    (PastMode::Convolution, "Spectral Convolution",
     "Spectral Convolution — per-bin self-resonance via complex convolution between the current bin and a delayed copy of itself (Delay = TIME curve). Builds artificial reverberant character without an external IR. Sidechain: not used."),
    (PastMode::Reverse,
     "Reverse",
     "Reverse — backward read of the per-bin history buffer. AMOUNT scales the reversed read; THRESHOLD gates by magnitude. Useful as a reverse-cymbal / backward-tape effect on sustains. Sidechain: not used."),
    (PastMode::Stretch,     "Stretch",
     "Stretch — phase-coherent variable-rate playback of the history buffer (0.25\u{00d7} – 4\u{00d7}). Slowing slot-history playback below 1\u{00d7} ratchets sustains; speeding up compresses tails. Sidechain: not used."),
];

pub const SORT_KEYS: &[(SortKey, &str, &str)] = &[
    (SortKey::Decay,     "Decay (ring time)", "Sort bins by how long they ring out — slowest-decaying bins move toward the loudest positions."),
    (SortKey::Stability, "Stability (IF)",    "Sort bins by instantaneous-frequency stability — most stable (tonal) bins move toward the loudest positions."),
    (SortKey::Area,      "Area (RMS)",        "Sort bins by recent RMS energy — loudest-on-average bins move toward the loudest positions."),
];

pub fn mode_label(mode: PastMode) -> &'static str {
    for &(m, label, _) in MODES {
        if m == mode { return label; }
    }
    "Unknown"
}
