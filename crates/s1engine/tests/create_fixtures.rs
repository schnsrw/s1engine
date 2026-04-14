//! Helper to generate test fixtures for fidelity validation.
//! Run with: cargo test --test create_fixtures -- --ignored

use s1engine::{Engine, Format};
use std::fs;
use std::path::Path;

fn engine() -> Engine {
    Engine::new()
}

#[test]
#[ignore] // Run manually to regenerate fixtures
fn generate_text_only_fixture() {
    let dir = Path::new("tests/fixtures");
    fs::create_dir_all(dir).unwrap();

    let content = "First paragraph with plain text.\n\
                   Second paragraph has more content here.\n\
                   Third paragraph is the final one.";
    let doc = engine().open_as(content.as_bytes(), Format::Txt).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    fs::write(dir.join("text-only.docx"), bytes).unwrap();
    println!("Created tests/fixtures/text-only.docx");
}

#[test]
#[ignore]
fn generate_formatted_fixture() {
    let dir = Path::new("tests/fixtures");
    fs::create_dir_all(dir).unwrap();

    let md = "# Heading One\n\n\
              Normal paragraph text.\n\n\
              ## Heading Two\n\n\
              Another normal paragraph.\n\n\
              **Bold text** and *italic text* mixed.\n\n\
              Final paragraph.";
    let doc = engine().open_as(md.as_bytes(), Format::Md).unwrap();
    let bytes = doc.export(Format::Docx).unwrap();
    fs::write(dir.join("formatted.docx"), bytes).unwrap();
    println!("Created tests/fixtures/formatted.docx");
}
