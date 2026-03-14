/// Creates three test DOCX documents for the editor:
///
///   1. formatted.docx   — headings, bold/italic/underline, colors, bullet list
///   2. with-table.docx  — heading, 3x3 table, trailing paragraph
///   3. multi-page.docx  — 20+ paragraphs to span multiple pages
///
/// Usage:
///   cargo run              (writes into current directory)
///   cargo run -- /some/dir (writes into the given directory)

use std::path::PathBuf;
use s1engine::{Color, DocumentBuilder, Format};

fn main() {
    let out_dir: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    std::fs::create_dir_all(&out_dir).unwrap();

    create_formatted(&out_dir);
    create_with_table(&out_dir);
    create_multi_page(&out_dir);

    println!("All test documents created in {}", out_dir.display());
}

/// formatted.docx — rich formatting showcase
fn create_formatted(dir: &PathBuf) {
    let doc = DocumentBuilder::new()
        .title("Formatted Test Document")
        .author("s1engine test suite")
        // Heading 1
        .heading(1, "Test Document")
        // Paragraph with bold, italic, underline
        .paragraph(|p| {
            p.text("This paragraph has ")
                .bold("bold text")
                .text(", ")
                .italic("italic text")
                .text(", and ")
                .underline("underlined text")
                .text(". It also has ")
                .bold_italic("bold-italic")
                .text(" combined.")
        })
        // Heading 2
        .heading(2, "Section Two")
        // Paragraph with styled and colored text
        .paragraph(|p| {
            p.text("Normal text followed by ")
                .styled("large Arial text", "Arial", 18.0)
                .text(" and ")
                .colored("red text", Color::from_hex("FF0000").unwrap())
                .text(" and ")
                .colored("blue text", Color::from_hex("0000FF").unwrap())
                .text(".")
        })
        // Heading 2
        .heading(2, "Bullet List")
        // Bullet list with 3 items
        .bullet("First bullet point item")
        .bullet("Second bullet point item")
        .bullet("Third bullet point item")
        .build();

    let path = dir.join("formatted.docx");
    let bytes = doc.export(Format::Docx).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    println!("  Created {} ({} bytes)", path.display(), bytes.len());
}

/// with-table.docx — table content
fn create_with_table(dir: &PathBuf) {
    let doc = DocumentBuilder::new()
        .title("Table Test Document")
        .author("s1engine test suite")
        .heading(1, "Table Test")
        .text("The following table contains sample data:")
        .table(|t| {
            t.row(|r| r.cell("Name").cell("Department").cell("Location"))
                .row(|r| r.cell("Alice Johnson").cell("Engineering").cell("New York"))
                .row(|r| r.cell("Bob Smith").cell("Marketing").cell("London"))
        })
        .text("This paragraph appears after the table.")
        .build();

    let path = dir.join("with-table.docx");
    let bytes = doc.export(Format::Docx).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    println!("  Created {} ({} bytes)", path.display(), bytes.len());
}

/// multi-page.docx — enough content to span multiple pages
fn create_multi_page(dir: &PathBuf) {
    let mut builder = DocumentBuilder::new()
        .title("Multi-Page Test Document")
        .author("s1engine test suite")
        .heading(1, "Multi-Page Document");

    // Generate 25 paragraphs of substantial text to fill multiple pages
    for i in 1..=25 {
        let heading_text = format!("Section {i}");
        let body_text = format!(
            "This is paragraph {i} of the multi-page test document. \
             It contains enough text to take up a reasonable amount of space \
             on the page. The purpose of this document is to test pagination \
             and scrolling behavior in the editor. Each paragraph is numbered \
             so you can verify that all content is present and in order. \
             Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do \
             eiusmod tempor incididunt ut labore et dolore magna aliqua. \
             Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris."
        );

        if i % 5 == 1 {
            // Add a heading every 5 paragraphs
            builder = builder.heading(2, &heading_text);
        }

        let body = body_text.clone();
        builder = builder.paragraph(move |p| p.text(&body));
    }

    let doc = builder.build();

    let path = dir.join("multi-page.docx");
    let bytes = doc.export(Format::Docx).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    println!("  Created {} ({} bytes)", path.display(), bytes.len());
}
