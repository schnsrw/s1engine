#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Must never panic, regardless of input
    let engine = s1engine::Engine::new();
    let _ = engine.open(data);
});
