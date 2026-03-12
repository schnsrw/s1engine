//! Performance benchmarks for s1engine core operations.
//!
//! Run with: cargo bench -p s1engine

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use s1engine::{DocumentBuilder, Engine, Format};

// ── Helpers ────────────────────────────────────────────────────────────

fn build_small_doc() -> s1engine::Document {
    DocumentBuilder::new()
        .title("Benchmark Doc")
        .heading(1, "Introduction")
        .text("This is a test paragraph with some content.")
        .paragraph(|p| p.text("Normal ").bold("bold").text(" and ").italic("italic"))
        .build()
}

fn build_medium_doc() -> s1engine::Document {
    let mut builder = DocumentBuilder::new().title("Medium Document");
    for i in 0..50 {
        builder = builder
            .heading(2, &format!("Section {}", i + 1))
            .text(&format!(
                "Paragraph {} with enough content to be realistic. \
                 Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
                i + 1
            ));
    }
    builder.build()
}

fn build_table_doc() -> s1engine::Document {
    DocumentBuilder::new()
        .heading(1, "Report")
        .table(|t| {
            let mut t = t.row(|r| r.cell("Name").cell("Value").cell("Status"));
            for i in 0..20 {
                let name = format!("Item {}", i + 1);
                let value = format!("{}", (i + 1) * 100);
                t = t.row(move |r| r.cell(&name).cell(&value).cell("OK"));
            }
            t
        })
        .build()
}

// ── Benchmarks ─────────────────────────────────────────────────────────

fn bench_create_empty(c: &mut Criterion) {
    let engine = Engine::new();
    c.bench_function("create_empty_document", |b| {
        b.iter(|| {
            let doc = engine.create();
            black_box(doc);
        });
    });
}

fn bench_builder_small(c: &mut Criterion) {
    c.bench_function("builder_small_doc", |b| {
        b.iter(|| {
            let doc = build_small_doc();
            black_box(doc);
        });
    });
}

fn bench_builder_medium(c: &mut Criterion) {
    c.bench_function("builder_medium_50_sections", |b| {
        b.iter(|| {
            let doc = build_medium_doc();
            black_box(doc);
        });
    });
}

fn bench_builder_table(c: &mut Criterion) {
    c.bench_function("builder_table_20_rows", |b| {
        b.iter(|| {
            let doc = build_table_doc();
            black_box(doc);
        });
    });
}

fn bench_to_plain_text(c: &mut Criterion) {
    let doc = build_medium_doc();
    c.bench_function("to_plain_text_50_sections", |b| {
        b.iter(|| {
            let text = doc.to_plain_text();
            black_box(text);
        });
    });
}

fn bench_export_docx_small(c: &mut Criterion) {
    let doc = build_small_doc();
    c.bench_function("export_docx_small", |b| {
        b.iter(|| {
            let bytes = doc.export(Format::Docx).unwrap();
            black_box(bytes);
        });
    });
}

fn bench_export_docx_medium(c: &mut Criterion) {
    let doc = build_medium_doc();
    c.bench_function("export_docx_50_sections", |b| {
        b.iter(|| {
            let bytes = doc.export(Format::Docx).unwrap();
            black_box(bytes);
        });
    });
}

fn bench_export_odt_small(c: &mut Criterion) {
    let doc = build_small_doc();
    c.bench_function("export_odt_small", |b| {
        b.iter(|| {
            let bytes = doc.export(Format::Odt).unwrap();
            black_box(bytes);
        });
    });
}

fn bench_export_txt(c: &mut Criterion) {
    let doc = build_medium_doc();
    c.bench_function("export_txt_50_sections", |b| {
        b.iter(|| {
            let bytes = doc.export(Format::Txt).unwrap();
            black_box(bytes);
        });
    });
}

fn bench_open_docx(c: &mut Criterion) {
    let doc = build_small_doc();
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();

    c.bench_function("open_docx_small", |b| {
        b.iter(|| {
            let doc = engine.open(black_box(&bytes)).unwrap();
            black_box(doc);
        });
    });
}

fn bench_open_docx_medium(c: &mut Criterion) {
    let doc = build_medium_doc();
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();

    c.bench_function("open_docx_50_sections", |b| {
        b.iter(|| {
            let doc = engine.open(black_box(&bytes)).unwrap();
            black_box(doc);
        });
    });
}

fn bench_open_odt(c: &mut Criterion) {
    let doc = build_small_doc();
    let bytes = doc.export(Format::Odt).unwrap();
    let engine = Engine::new();

    c.bench_function("open_odt_small", |b| {
        b.iter(|| {
            let doc = engine.open(black_box(&bytes)).unwrap();
            black_box(doc);
        });
    });
}

fn bench_roundtrip_docx(c: &mut Criterion) {
    let doc = build_small_doc();
    let bytes = doc.export(Format::Docx).unwrap();
    let engine = Engine::new();

    c.bench_function("roundtrip_docx_small", |b| {
        b.iter(|| {
            let doc = engine.open(black_box(&bytes)).unwrap();
            let out = doc.export(Format::Docx).unwrap();
            black_box(out);
        });
    });
}

fn bench_undo_redo(c: &mut Criterion) {
    use s1_ops::Operation;

    c.bench_function("undo_redo_10_ops", |b| {
        b.iter(|| {
            let mut doc = build_small_doc();
            let para_ids = doc.paragraph_ids();
            if para_ids.is_empty() {
                return;
            }

            // Find a text node to edit
            let para = doc.node(para_ids[0]).unwrap();
            if para.children.is_empty() {
                return;
            }
            let run_id = para.children[0];
            let run = doc.node(run_id).unwrap();
            if run.children.is_empty() {
                return;
            }
            let text_id = run.children[0];

            // Apply 10 insert operations
            for i in 0..10 {
                let op = Operation::InsertText {
                    target_id: text_id,
                    offset: i,
                    text: "x".to_string(),
                };
                doc.apply(op).unwrap();
            }

            // Undo all 10
            for _ in 0..10 {
                doc.undo().unwrap();
            }

            // Redo all 10
            for _ in 0..10 {
                doc.redo().unwrap();
            }

            black_box(&doc);
        });
    });
}

fn bench_format_detection(c: &mut Criterion) {
    let doc = build_small_doc();
    let docx_bytes = doc.export(Format::Docx).unwrap();
    let odt_bytes = doc.export(Format::Odt).unwrap();
    let txt_bytes = doc.export(Format::Txt).unwrap();

    c.bench_function("format_detection", |b| {
        b.iter(|| {
            black_box(Format::detect(&docx_bytes));
            black_box(Format::detect(&odt_bytes));
            black_box(Format::detect(&txt_bytes));
        });
    });
}

criterion_group!(
    benches,
    bench_create_empty,
    bench_builder_small,
    bench_builder_medium,
    bench_builder_table,
    bench_to_plain_text,
    bench_export_docx_small,
    bench_export_docx_medium,
    bench_export_odt_small,
    bench_export_txt,
    bench_open_docx,
    bench_open_docx_medium,
    bench_open_odt,
    bench_roundtrip_docx,
    bench_undo_redo,
    bench_format_detection,
);
criterion_main!(benches);
