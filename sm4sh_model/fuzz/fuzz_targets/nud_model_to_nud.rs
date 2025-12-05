#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: sm4sh_model::NudModel| {
    // Check for panics.
    let _ = input.to_nud();
});
