//! Simple document converter CLI.
//!
//! Usage: cargo run --example convert -- input.docx output.odt
//!
//! Demonstrates:
//! - Opening documents from file
//! - Format detection from extension
//! - Exporting to a different format
//! - Querying document metadata and structure

use s1engine::{Engine, Format};
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        eprintln!();
        eprintln!("Supported formats: .docx, .odt, .txt, .pdf (export only)");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  {} report.docx report.odt", args[0]);
        eprintln!("  {} report.docx report.txt", args[0]);
        eprintln!("  {} report.docx report.pdf", args[0]);
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    // Detect output format from extension
    let output_format = match Format::from_path(Path::new(output_path)) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    // Open the input document
    let engine = Engine::new();
    let doc = match engine.open_file(input_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error opening {input_path}: {e}");
            process::exit(1);
        }
    };

    // Print document info
    let meta = doc.metadata();
    if let Some(title) = &meta.title {
        println!("Title:      {title}");
    }
    if let Some(creator) = &meta.creator {
        println!("Author:     {creator}");
    }
    println!("Paragraphs: {}", doc.paragraph_count());
    println!("Text length: {} chars", doc.to_plain_text().len());
    println!();

    // Export to the target format
    let output_bytes = match doc.export(output_format) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error exporting to {output_format:?}: {e}");
            process::exit(1);
        }
    };

    // Write output
    if let Err(e) = std::fs::write(output_path, &output_bytes) {
        eprintln!("Error writing {output_path}: {e}");
        process::exit(1);
    }

    println!(
        "Converted {} -> {} ({} bytes)",
        input_path,
        output_path,
        output_bytes.len()
    );
}
