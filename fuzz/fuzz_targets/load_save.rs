#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(envelope) = rmp_serde::from_slice::<persistence::SaveEnvelope>(data) {
        let _ = envelope.decode();
    }
});
