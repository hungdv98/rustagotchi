#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let _ = ron::from_str::<content::definitions::DefinitionList<
            content::definitions::MoveDefinition,
        >>(source);
    }
});
