//! Phase 1 audit harness: drive a Pipeline with all-Empty slots and
//! continuous noise, capture state-container max magnitudes after a short
//! and long run, and flag any container that grew significantly between.
//!
//! This test is `#[ignore]`d by default. The audit's primary deliverable
//! is the written report in docs/superpowers/2026-05-06-phase-1-diagnostics.md
//! Run manually with:
//!   cargo test --features=probe --test empty_slot_smear_audit -- --include-ignored --nocapture

#[test]
#[ignore]
fn audit_state_growth_under_empty_slot_noise() {
    println!("Phase 1 audit harness — see Task 3 of stabilization plan.");
    println!("Static audit's primary deliverable is the written report at");
    println!("docs/superpowers/2026-05-06-phase-1-diagnostics.md");
    println!();
    println!("Primary candidate identified: prev_unwrapped_phase[ch] + total_hops_per_ch");
    println!("  File:  src/dsp/pipeline.rs:1084-1116 (plpv.rs:69)");
    println!("  Cause: f32 precision loss in cumulative phase accumulator when plpv_enable=true");
    println!("  Grows: two_pi_hop_over_n * k * hop_total — reaches ~2.78e8 at 30 min, k=1024");
    println!("  Reset: power-cycle calls Pipeline::new(), zeroing total_hops_per_ch and");
    println!("         prev_unwrapped_phase. GUI Reset (clear_state) does NOT reset these.");
    println!();
    println!("If the static audit didn't pin the smearing accumulator, the");
    println!("Task 11 implementer can extend this test with probe-feature");
    println!("instrumentation to identify it empirically.");
    println!();
    println!("Proposed fix: periodic reset of prev_unwrapped_phase + total_hops_per_ch");
    println!("every M=4096 hops (Option A), or fractional-residual accumulation (Option B).");
    println!("See §2 of the diagnostic report for full analysis.");
}
