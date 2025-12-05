#![no_main]

use libfuzzer_sys::fuzz_target;

#[derive(Debug, arbitrary::Arbitrary)]
struct Input {
    nud: sm4sh_lib::nud::Nud,
    nut: Option<sm4sh_lib::nut::Nut>,
    vbn: Option<sm4sh_lib::vbn::Vbn>,
}

fuzz_target!(|input: Input| {
    // Check for panics.
    let _ = sm4sh_model::NudModel::from_nud(&input.nud, input.nut.as_ref(), input.vbn.as_ref());
});
