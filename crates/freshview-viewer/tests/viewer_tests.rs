use std::path::Path;

use freshview_viewer::document::ViewerDocument;

fn test_pdf_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test.pdf")
}

#[test]
fn test_open_pdf_page_count() {
    let test_pdf = test_pdf_path();
    if !test_pdf.exists() {
        return;
    }
    let doc = ViewerDocument::open(&test_pdf).unwrap();
    assert!(doc.page_count() > 0);
}

#[test]
fn test_render_page_to_rgba() {
    let test_pdf = test_pdf_path();
    if !test_pdf.exists() {
        return;
    }
    let doc = ViewerDocument::open(&test_pdf).unwrap();
    let (rgba, width, height) = doc.render_page(0, 1.0).unwrap();
    assert!(width > 0);
    assert!(height > 0);
    assert_eq!(rgba.len(), (width * height * 4) as usize);
}

#[test]
fn test_render_page_zoom() {
    let test_pdf = test_pdf_path();
    if !test_pdf.exists() {
        return;
    }
    let doc = ViewerDocument::open(&test_pdf).unwrap();
    let (_rgba1, w1, h1) = doc.render_page(0, 1.0).unwrap();
    let (_rgba2, w2, h2) = doc.render_page(0, 2.0).unwrap();
    // At 2x zoom, dimensions should be roughly double
    assert!(w2 > w1);
    assert!(h2 > h1);
}

#[test]
fn test_open_nonexistent_file_errors() {
    let result = ViewerDocument::open(Path::new("/nonexistent/file.pdf"));
    assert!(result.is_err());
}

#[test]
fn test_render_invalid_page_index_errors() {
    let test_pdf = test_pdf_path();
    if !test_pdf.exists() {
        return;
    }
    let doc = ViewerDocument::open(&test_pdf).unwrap();
    let result = doc.render_page(999, 1.0);
    assert!(result.is_err());
}
