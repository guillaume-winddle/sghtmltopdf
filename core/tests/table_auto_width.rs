//! E2E tests for the width of a table with `width: auto`.
//!
//! Such a table is as wide as its columns, up to the containing width (CSS2.1 17.5.2.2).
//! Before, it always filled the containing width, as a block does. The expected values were
//! measured in Chrome on the same HTML and CSS with DejaVu Sans, rounded to the pixel. Every
//! case sets the cell padding and `border-spacing`, so the user-agent defaults do not matter.

use sghtmltopdf::fonts::{Font, FontCollection};
use sghtmltopdf::html;
use sghtmltopdf::layout::{build_box_tree, layout_document, LaidOutBox, LaidOutContent};
use sghtmltopdf::style::{compute_styles, parse_stylesheet, user_agent_stylesheet};

const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fonts/DejaVuSans.ttf");

const CSS: &str = "body { margin: 0; font-size: 14px; } \
                   table { border-spacing: 0; } td { padding: 4px; }";

/// A laid-out table: its border box x and width, and the border box widths of its first row.
struct Table {
    x: f32,
    width: f32,
    cells: Vec<f32>,
}

/// Lay `html_src` out in a 700px wide containing block and return its tables in document order.
fn tables(html_src: &str, css: &str) -> Vec<Table> {
    let dom = html::parse(html_src.as_bytes());
    let author = parse_stylesheet(&format!("{CSS} {css}"));
    let styles = compute_styles(&dom, &user_agent_stylesheet(), &author);
    let fonts = FontCollection::new(vec![
        Font::load(FONT_PATH).expect("should load bundled test font")
    ]);
    let tree = build_box_tree(&dom, &styles);
    let laid = layout_document(&tree, &styles, &fonts, 700.0);

    fn walk(b: &LaidOutBox, out: &mut Vec<Table>) {
        match &b.content {
            LaidOutContent::Table(table) => {
                out.push(Table {
                    x: b.layout.border_box().x,
                    width: b.layout.border_box().width,
                    cells: table.rows[0]
                        .cells
                        .iter()
                        .map(|c| c.layout.border_box().width)
                        .collect(),
                });
                for cell in table.rows.iter().flat_map(|row| &row.cells) {
                    walk(cell, out);
                }
            }
            LaidOutContent::Blocks(children) | LaidOutContent::Flex(children) => {
                children.iter().for_each(|child| walk(child, out));
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&laid, &mut out);
    out
}

fn assert_px(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 0.5,
        "{what}: expected {expected}px, got {actual}px"
    );
}

fn assert_cells(table: &Table, expected: &[f32]) {
    assert_eq!(table.cells.len(), expected.len());
    for (actual, expected) in table.cells.iter().zip(expected) {
        assert_px(*actual, *expected, "cell width");
    }
}

#[test]
fn an_auto_width_table_is_as_wide_as_its_columns() {
    let t = &tables("<table><tr><td>Key:</td><td>Value</td></tr></table>", "")[0];
    assert_px(t.width, 84.0, "table width");
    assert_cells(t, &[37.0, 46.0]);
}

#[test]
fn border_spacing_is_part_of_the_auto_width() {
    let t = &tables(
        "<table><tr><td>Key:</td><td>Value</td></tr></table>",
        "table { border-spacing: 10px; }",
    )[0];
    assert_px(t.width, 114.0, "table width");
    assert_cells(t, &[37.0, 46.0]);
}

#[test]
fn the_table_padding_and_border_are_added_to_its_columns() {
    let t = &tables(
        "<table><tr><td>Key:</td><td>Value</td></tr></table>",
        "table { padding: 10px; border: 3px solid; }",
    )[0];
    assert_px(t.width, 110.0, "table width");
    assert_cells(t, &[37.0, 46.0]);
}

#[test]
fn auto_margins_centre_an_auto_width_table() {
    let t = &tables(
        "<table><tr><td>Key:</td><td>Value</td></tr></table>",
        "table { margin: 0 auto; }",
    )[0];
    assert_px(t.width, 84.0, "table width");
    assert_px(t.x, (700.0 - t.width) / 2.0, "table x");
}

#[test]
fn a_table_wider_than_its_container_still_fits_it() {
    let t = &tables(
        r#"<div style="width: 300px"><table><tr>
             <td>Key:</td><td>A value that is much too long to fit on one line in the box</td>
           </tr></table></div>"#,
        "",
    )[0];
    assert_px(t.width, 300.0, "table width");
    assert_cells(t, &[37.0, 263.0]);
}

#[test]
fn a_table_with_a_width_still_fills_it() {
    let t = &tables(
        "<table><tr><td>Key:</td><td>Value</td></tr></table>",
        "table { width: 100%; }",
    )[0];
    assert_px(t.width, 700.0, "table width");
}

#[test]
fn the_widest_cell_of_each_column_counts_whatever_its_row() {
    let t = &tables(
        r#"<table>
             <tr><td>A long first cell</td><td>b</td></tr>
             <tr><td>c</td><td>A long second cell</td></tr>
           </table>"#,
        "",
    )[0];
    assert_px(t.width, 248.0, "table width");
    assert_cells(t, &[113.0, 135.0]);
}

#[test]
fn the_caption_min_content_width_is_a_floor() {
    let t = &tables(
        r#"<table><caption>A_caption_wider_than_the_cells</caption>
             <tr><td>a</td><td>b</td></tr></table>"#,
        "",
    )[0];
    assert_px(t.width, 221.0, "table width");
}

#[test]
fn a_cell_width_in_px_sets_its_column_width() {
    let t = &tables(
        r#"<table><tr><td style="width: 200px">a</td><td>b</td></tr></table>"#,
        "",
    )[0];
    assert_px(t.width, 225.0, "table width");
    assert_cells(t, &[208.0, 17.0]);
}

#[test]
fn a_col_width_in_px_counts_in_the_auto_width() {
    let t = &tables(
        r#"<table><colgroup><col style="width: 300px"><col></colgroup>
             <tr><td>a</td><td>b</td></tr></table>"#,
        "",
    )[0];
    assert_px(t.width, 317.0, "table width");
    assert_cells(t, &[300.0, 17.0]);
}

#[test]
fn a_nested_auto_width_table_is_as_wide_as_its_columns() {
    let found = tables(
        r#"<table><tr>
             <td><table><tr><td>Key:</td><td>Value</td></tr></table></td><td>x</td>
           </tr></table>"#,
        "",
    );
    assert_px(found[0].width, 108.0, "outer table width");
    assert_cells(&found[0], &[92.0, 16.0]);
    assert_px(found[1].width, 84.0, "inner table width");
}

#[test]
fn a_display_table_element_is_as_wide_as_its_columns() {
    let t = &tables(
        r#"<div class="t"><div class="c">Key:</div><div class="c">Value</div></div>"#,
        ".t { display: table; } .c { display: table-cell; padding: 4px; }",
    )[0];
    assert_px(t.width, 84.0, "table width");
}
