use s1_format_docy::DocyWriter;

#[test]
fn compare_pptx_binary() {
    let mut w = DocyWriter::new();
    // Simulate what our image writer produces for a 50x50mm image with data URL
    let data_url = "data:image/png;base64,TEST";
    let cx = 50u32 * 36000; // 50mm in EMU-like units
    let cy = 50u32 * 36000;
    
    crate::content::image::write_pptx_picture_binary_test(&mut w, cx, cy, data_url);
    
    let our_bytes = w.as_bytes();
    println!("Our output: {} bytes", our_bytes.len());
    
    // Load captured reference
    let ref_bytes = std::fs::read("/tmp/pptx_sample.bin").unwrap_or_default();
    println!("Reference: {} bytes", ref_bytes.len());
    
    // Compare structure (first 50 bytes of each)
    println!("\nOur first 50 bytes:");
    for i in 0..50.min(our_bytes.len()) {
        if i % 20 == 0 { print!("\n  {:4}: ", i); }
        print!("{:02x} ", our_bytes[i]);
    }
    println!("\n\nRef first 50 bytes:");
    for i in 0..50.min(ref_bytes.len()) {
        if i % 20 == 0 { print!("\n  {:4}: ", i); }
        print!("{:02x} ", ref_bytes[i]);
    }
    println!();
    
    // Find first difference
    let min_len = our_bytes.len().min(ref_bytes.len());
    for i in 0..min_len {
        if our_bytes[i] != ref_bytes[i] {
            println!("\nFirst diff at byte {}: ours=0x{:02x} ref=0x{:02x}", i, our_bytes[i], ref_bytes[i]);
            break;
        }
    }
}
