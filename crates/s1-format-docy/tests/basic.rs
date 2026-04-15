use s1_format_docy;
use s1_model::DocumentModel;

#[test]
fn empty_model_produces_valid_docy() {
    let model = DocumentModel::new();
    let docy = s1_format_docy::write(&model);
    assert!(docy.starts_with("DOCY;v5;"));
    assert!(docy.len() > 20, "DOCY should have content: {}", docy.len());
}

#[test]
fn docy_header_format() {
    let model = DocumentModel::new();
    let docy = s1_format_docy::write(&model);
    let parts: Vec<&str> = docy.splitn(4, ';').collect();
    assert_eq!(parts[0], "DOCY");
    assert_eq!(parts[1], "v5");
    let size: usize = parts[2].parse().expect("size should be a number");
    assert!(size > 0, "binary size should be > 0");
    assert!(!parts[3].is_empty(), "base64 data should not be empty");
}
