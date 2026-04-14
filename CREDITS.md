# Credits and Acknowledgements

## Frameworks and libraries

**nih-plug** — Robbert van der Helm  
https://github.com/robbert-vdh/nih-plug  
MIT License  
Plugin lifecycle, CLAP/VST3 integration, `StftHelper` overlap-add engine, egui integration (`nih_plug_egui`), and parameter smoothing. The `StftHelper` is the direct basis for the STFT pipeline in this project.

**realfft** — Henrik Enquist  
https://github.com/HEnquist/realfft  
MIT License  
Real-to-complex and complex-to-real FFT used for the STFT analysis and synthesis windows. Enforces Hermitian symmetry on the complex bin array.

**triple_buffer** — Hadrien G.  
https://github.com/HadrienG2/triple-buffer  
MIT License  
Lock-free triple buffering for GUI↔audio thread communication (spectrum display data and curve parameter updates).

**CLAP plugin standard** — Alexandre Bique and contributors  
https://github.com/free-audio/clap  
MIT License  
The plugin ABI that Spectral Forge targets. The standard was created in collaboration between Bitwig and u-he.

**egui** — Emil Ernerfeldt and contributors  
https://github.com/emilk/egui  
MIT / Apache 2.0  
Immediate-mode GUI used for all rendering, via `nih_plug_egui`.

**parking_lot** — Amanieu d'Antras  
https://github.com/Amanieu/parking_lot  
MIT / Apache 2.0  
Mutex used for GUI-state shared data.

**num-complex** — The Rust num developers  
https://github.com/rust-num/num-complex  
MIT / Apache 2.0  
Complex number arithmetic for FFT bins.

---

## Algorithm references

**pvx** — Colby Leider and contributors  
https://github.com/TheColby/pvx  
MIT License  
Python phase-vocoder and STFT toolkit, consulted as a reference during development. Specifically used to verify correct handling of DC and Nyquist bin Hermitian symmetry constraints (which resolved the PhaseRand and Freeze crashes), and to review best-practice approaches to spectral warbling suppression in overlap-add processors.  
No source code from pvx was copied into this project.

**Xorshift random number generators** — George Marsaglia  
*Journal of Statistical Software*, Vol. 8, Issue 14 (2003)  
https://www.jstatsoft.org/article/view/v008i14  
Public domain  
The xorshift64 PRNG used by the phase randomiser (`state ^= state << 13; state ^= state >> 7; state ^= state << 17`) is the three-rotation xorshift variant from this paper.

**Overlap-add normalisation for Hann windows** — Griffin & Lim (1984)  
*IEEE Transactions on Acoustics, Speech, and Signal Processing*, 32(2)  
The Hann² COLA (Constant Overlap-Add) condition at 75% overlap gives the normalisation constant `2 / (3 × FFT_SIZE)` used throughout the STFT pipeline.

**Soft knee gain computer** — standard compressor design  
The quadratic soft-knee formula used in the spectral compressor is the formulation described in Zölzer, *DAFX: Digital Audio Effects* (2nd ed., Wiley, 2011), Chapter 4.

**Gaussian bell parametric EQ** — standard parametric EQ formulation  
The bell filter shape used for curve nodes is a frequency-domain Gaussian bell, a common approximation to constant-Q peak filters in spectral equaliser literature.

**Smoothstep shelf transition** — standard technique  
The S-curve `3t² − 2t³` (smoothstep) used for shelf filter transitions is a standard polynomial from computer graphics, applied here to create a smooth gain ramp between the shelf's flat regions.

**Prefix-sum log-frequency box filter** — standard signal processing  
The O(N) prefix-sum approach for blurring gain-reduction masks across a log-frequency window is a standard technique used in spectral noise suppression (Ephraim & Malah, 1984) and implemented here for both the compressor GR mask and the contrast local-mean computation.
