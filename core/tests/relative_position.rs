//! End-to-end tests for `position: relative` (waka/sghtmltopdf#29).
//!
//! The offset has to move the text, images, `<span>`s and child blocks as well as
//! the background and border. These run the same document as the reproduction in
//! the issue (`@page { margin: 0.5in }`, `* { margin: 0; padding: 0 }`) all the
//! way through pagination, and check that the decoration rectangle (border box)
//! and the coordinates of the contents agree.

use std::path::PathBuf;

use sghtmltopdf::fonts::{Font, FontCollection};
use sghtmltopdf::html::{self, Dom, NodeData, NodeId};
use sghtmltopdf::img::{DocumentImageCache, ImageFetcher};
use sghtmltopdf::layout::{paginate_document, LaidOutBox, LaidOutContent, PageSettings};
use sghtmltopdf::style::{compute_styles, extract_author_stylesheet, user_agent_stylesheet};

const FONT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fonts/DejaVuSans.ttf");

fn test_fonts() -> FontCollection {
    FontCollection::new(vec![
        Font::load(FONT_PATH).expect("should load bundled test font")
    ])
}

fn find(dom: &Dom, id: NodeId, tag: &str) -> Option<NodeId> {
    if let NodeData::Element { name, .. } = &dom.node(id).data {
        if &*name.local == tag {
            return Some(id);
        }
    }
    dom.children(id).find_map(|child| find(dom, child, tag))
}

fn find_laid_out(b: &LaidOutBox, target: NodeId) -> Option<&LaidOutBox> {
    if b.node == Some(target) {
        return Some(b);
    }
    if let LaidOutContent::Blocks(children) = &b.content {
        for child in children {
            if let Some(found) = find_laid_out(child, target) {
                return Some(found);
            }
        }
    }
    None
}

/// Lays out the same document as the `probe` in the issue up to the first page,
/// and hands the layout of the first `tag` element to `f`.
fn with_probe<T>(css: &str, body: &str, tag: &str, f: impl FnOnce(&LaidOutBox) -> T) -> T {
    let html_src = format!(
        "<!DOCTYPE html><html><head><style>\
         @page {{ size: letter; margin: 0.5in }} * {{ margin: 0; padding: 0 }} {css}\
         </style></head><body>{body}</body></html>"
    );
    let dom = html::parse(html_src.as_bytes());
    let fetcher = ImageFetcher::new(PathBuf::from("."), false);
    let cache = DocumentImageCache::new();
    let author = extract_author_stylesheet(&dom, &fetcher, &cache);
    let styles = compute_styles(&dom, &user_agent_stylesheet(), &author);
    let fonts = test_fonts();
    let pages = paginate_document(&dom, &styles, &fonts, &PageSettings::default());
    let node = find(&dom, dom.document(), tag).expect("probe element not found");
    let b = pages[0]
        .boxes
        .iter()
        .find_map(|b| find_laid_out(b, node))
        .expect("probe element not laid out on page 1");
    f(b)
}

fn first_line(b: &LaidOutBox) -> &sghtmltopdf::layout::LineBox {
    let LaidOutContent::Inline(lines) = &b.content else {
        panic!("expected inline content");
    };
    &lines[0]
}

const OFFSET: f32 = 120.0;

#[test]
fn text_moves_with_the_background_of_a_relative_block() {
    with_probe(
        ".b { position: relative; left: 120px; background: #ffd; width: 160px }",
        r#"<div class="b">TEXT</div>"#,
        "div",
        |b| {
            let line = first_line(b);
            assert_eq!(b.layout.border_box().x, OFFSET);
            assert_eq!(line.rect.x, b.layout.content.x);
        },
    );
}

#[test]
fn text_moves_with_the_border_of_a_relative_block() {
    with_probe(
        ".b { position: relative; left: 120px; border: 2px solid #000; width: 160px }",
        r#"<div class="b">TEXT</div>"#,
        "div",
        |b| {
            assert_eq!(b.layout.border_box().x, OFFSET);
            assert_eq!(first_line(b).rect.x, OFFSET + 2.0);
        },
    );
}

#[test]
fn a_nested_block_child_moves_with_its_relative_parent() {
    with_probe(
        ".b { position: relative; left: 120px; background: #ffd; width: 160px }",
        r#"<div class="b"><p>TEXT</p></div>"#,
        "p",
        |p| {
            assert_eq!(p.layout.content.x, OFFSET);
            assert_eq!(first_line(p).rect.x, OFFSET);
        },
    );
}

#[test]
fn an_image_moves_with_its_relative_parent() {
    with_probe(
        ".b { position: relative; left: 120px; width: 160px }",
        r#"<div class="b"><img src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=" style="width:40px;height:40px"></div>"#,
        "div",
        |b| {
            let line = first_line(b);
            assert_eq!(line.rect.x, OFFSET);
            let img = line
                .atomics
                .first()
                .expect("img should be an atomic inline");
            assert_eq!(img.content.layout.content.x, OFFSET);
        },
    );
}

#[test]
fn top_moves_the_text_with_the_background() {
    with_probe(
        ".b { position: relative; top: 40px; background: #ffd; width: 160px }",
        r#"<div class="b">TEXT</div>"#,
        "div",
        |b| {
            assert_eq!(b.layout.border_box().y, 40.0);
            assert_eq!(first_line(b).rect.y, 40.0);
        },
    );
}

#[test]
fn a_relative_span_moves_its_own_text_only() {
    with_probe(
        ".s { position: relative; left: 120px }",
        r#"<div>A <span class="s">TEXT</span></div>"#,
        "div",
        |b| {
            let line = first_line(b);
            let a = line.runs.iter().find(|r| r.text == "A").expect("run A");
            let text = line
                .runs
                .iter()
                .find(|r| r.text == "TEXT")
                .expect("run TEXT");
            assert_eq!(a.x_offset, 0.0);
            // The run would normally sit right after "A " (a.width plus the
            // space); it moves 120px to the right.
            assert!(
                text.x_offset > a.width + OFFSET && text.x_offset < a.width + OFFSET + 10.0,
                "TEXT run should sit 120px right of its normal position, got {}",
                text.x_offset
            );
        },
    );
}

/// Paginates the document and returns, page by page, the border box top of every `tag` element
/// on that page (a box split over pages counts on each).
fn tops_per_page(css: &str, body: &str, tag: &str) -> Vec<Vec<f32>> {
    let html_src = format!(
        "<!DOCTYPE html><html><head><style>\
         @page {{ size: letter; margin: 0.5in }} * {{ margin: 0; padding: 0 }} {css}\
         </style></head><body>{body}</body></html>"
    );
    let dom = html::parse(html_src.as_bytes());
    let fetcher = ImageFetcher::new(PathBuf::from("."), false);
    let cache = DocumentImageCache::new();
    let author = extract_author_stylesheet(&dom, &fetcher, &cache);
    let styles = compute_styles(&dom, &user_agent_stylesheet(), &author);
    let fonts = test_fonts();
    let pages = paginate_document(&dom, &styles, &fonts, &PageSettings::default());

    fn collect(dom: &Dom, id: NodeId, tag: &str, out: &mut Vec<NodeId>) {
        if let NodeData::Element { name, .. } = &dom.node(id).data {
            if &*name.local == tag {
                out.push(id);
            }
        }
        dom.children(id)
            .for_each(|child| collect(dom, child, tag, out));
    }
    fn tops(b: &LaidOutBox, ids: &[NodeId], out: &mut Vec<f32>) {
        if b.node.is_some_and(|node| ids.contains(&node)) {
            out.push(b.layout.border_box().y);
        }
        if let LaidOutContent::Blocks(children) = &b.content {
            children.iter().for_each(|child| tops(child, ids, out));
        }
    }
    let mut ids = Vec::new();
    collect(&dom, dom.document(), tag, &mut ids);
    pages
        .iter()
        .map(|page| {
            let mut out = Vec::new();
            page.boxes.iter().for_each(|b| tops(b, &ids, &mut out));
            out
        })
        .collect()
}

#[test]
fn a_relative_box_after_a_forced_page_break_keeps_its_offset() {
    // One label per page, moved down by `top`: each page draws it at the same place.
    let pages = tops_per_page(
        "section { position: relative; top: 100px; height: 200px } section + section { break-before: page }",
        "<section>1</section><section>2</section><section>3</section>",
        "section",
    );
    assert_eq!(pages.len(), 3);
    assert_eq!(pages[0], vec![100.0]);
    assert_eq!(pages[1], pages[0], "page 2");
    assert_eq!(pages[2], pages[0], "page 3");
}

#[test]
fn a_relative_box_pushed_to_the_next_page_keeps_its_offset() {
    // Two boxes do not fit on a letter page (912px of content): the second moves to page 2.
    let pages = tops_per_page(
        "section { position: relative; top: 30px; height: 500px }",
        "<section>1</section><section>2</section>",
        "section",
    );
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0], vec![30.0]);
    assert_eq!(pages[1], vec![30.0]);
}

#[test]
fn a_relative_offset_after_a_page_break_does_not_move_the_next_box() {
    let pages = tops_per_page(
        "section { height: 200px } .rel { position: relative; top: 100px } .rel { break-before: page }",
        r#"<section>1</section><section class="rel">2</section><section>3</section>"#,
        "section",
    );
    assert_eq!(pages.len(), 2);
    // The offset moves the second box only: the third follows it at its place in the flow.
    assert_eq!(pages[1], vec![100.0, 200.0]);
}
