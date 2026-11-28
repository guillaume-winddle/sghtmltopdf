//! E2E tests for `--enable-smart-shrinking` (`EngineOptions::smart_shrinking`).
//!
//! A document wider than the page area is laid out on a page widened to its content and drawn
//! at the scale that keeps the paper size, as wkhtmltopdf's smart shrinking does. The PDFs are
//! written uncompressed, so the page scale (the first `cm` of a page) and the text positions
//! (`Tm`, in CSS px before that scale) can be read back.

use sghtmltopdf::engine::{Engine, EngineError, EngineOptions, FontSpec, Mode};
use sghtmltopdf::sink::MemorySink;

const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fonts/DejaVuSans.ttf");

/// A table of `columns` columns that cannot wrap (about 110px each), on an A4 page (794px wide)
/// whose content area is 602px wide.
fn wide_table_of(columns: usize) -> String {
    let cells: String = (0..columns)
        .map(|i| format!(r#"<td style="white-space: nowrap">Column {i} value</td>"#))
        .collect();
    format!("<h1>Wide</h1><table><tr>{cells}</tr></table><p>After the table</p>")
}

/// About 1000px: it fits once the page is laid out 1.7 times wider.
fn wide_table() -> String {
    wide_table_of(9)
}

fn render(html: &str, smart_shrinking: bool, mode: Mode) -> Result<Vec<u8>, String> {
    let mut options = EngineOptions::default();
    options.fonts = vec![FontSpec {
        path: FONT_PATH.into(),
        index: 0,
    }];
    options.output.compress = false;
    options.smart_shrinking = smart_shrinking;
    options.mode = mode;
    let mut engine = Engine::new(options, MemorySink::new());
    engine
        .feed(html.as_bytes())
        .and_then(|()| engine.finish())
        .map_err(|e: EngineError<_>| e.to_string())
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The first page's `/MediaBox`, in points.
fn media_box(pdf: &str) -> Vec<f32> {
    let start = pdf.find("/MediaBox [").expect("a MediaBox") + "/MediaBox [".len();
    pdf[start..start + pdf[start..].find(']').unwrap()]
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect()
}

/// The scale of the first page: the first `s 0 0 s 0 0 cm` of its content.
fn page_scale(pdf: &str) -> f32 {
    pdf.lines()
        .find_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            (parts.len() == 7 && parts[6] == "cm" && parts[1] == "0" && parts[2] == "0")
                .then(|| parts[0].parse().ok())
                .flatten()
        })
        .expect("a page scale")
}

/// The x of every text line start, in CSS px (`1 0 0 1 x y Tm`).
fn text_starts(pdf: &str) -> Vec<f32> {
    pdf.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            (parts.len() == 7 && parts[6] == "Tm")
                .then(|| parts[4].parse().ok())
                .flatten()
        })
        .collect()
}

#[test]
fn a_document_wider_than_the_page_is_shrunk_to_fit_the_paper() {
    let off = text(&render(&wide_table(), false, Mode::Batch).unwrap());
    let on = text(&render(&wide_table(), true, Mode::Batch).unwrap());

    // The paper does not change, the scale does.
    for (on, off) in media_box(&on).iter().zip(media_box(&off)) {
        assert!((on - off).abs() < 0.01, "paper {on} instead of {off}");
    }
    assert!((page_scale(&off) - 0.75).abs() < 1e-4);
    let scale = page_scale(&on);
    assert!(
        scale < 0.75 / 1.5 && scale > 0.75 / 2.0,
        "the table is 1.5 to 2 times the content width: scale {scale}"
    );

    // Without it the last columns start past the paper's right edge, with it every text does not.
    let paper_width = 595.275;
    assert!(text_starts(&off).iter().any(|x| x * 0.75 > paper_width));
    for x in text_starts(&on) {
        assert!(
            x * scale < paper_width,
            "text at {x}px drawn past the paper"
        );
    }
}

#[test]
fn the_page_is_laid_out_at_most_twice_as_wide_as_the_paper() {
    // About 2200px of columns: the page is laid out 1588px wide (twice A4) and the rest is cut.
    let pdf = text(&render(&wide_table_of(20), true, Mode::Batch).unwrap());
    assert!(
        (page_scale(&pdf) - 0.75 / 2.0).abs() < 1e-3,
        "scale {}",
        page_scale(&pdf)
    );
}

#[test]
fn content_clipped_by_its_box_does_not_shrink_the_page() {
    // The wide table sits in a box that clips it: nothing is drawn past the page area.
    let html = format!(r#"<div style="overflow: hidden">{}</div>"#, wide_table());
    let off = text(&render(&html, false, Mode::Batch).unwrap());
    let on = text(&render(&html, true, Mode::Batch).unwrap());
    assert_eq!(page_scale(&on), page_scale(&off));
}

#[test]
fn a_document_that_fits_is_not_shrunk() {
    let html = "<h1>Narrow</h1><table><tr><td>a</td><td>b</td></tr></table>";
    let off = text(&render(html, false, Mode::Batch).unwrap());
    let on = text(&render(html, true, Mode::Batch).unwrap());
    assert_eq!(page_scale(&on), page_scale(&off));
    assert_eq!(text_starts(&on), text_starts(&off));
}

#[test]
fn smart_shrinking_is_refused_in_streaming_mode() {
    let error = render(&wide_table(), true, Mode::Streaming).unwrap_err();
    assert!(error.contains("--enable-smart-shrinking"), "{error}");
}
