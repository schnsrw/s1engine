#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Must never panic, regardless of input.
    // If we can open it, try to export in every format.
    let engine = s1engine::Engine::new();
    if let Ok(doc) = engine.open(data) {
        let _ = doc.export(s1engine::Format::Docx);
        let _ = doc.export(s1engine::Format::Odt);
        let _ = doc.export(s1engine::Format::Txt);
    }
});
