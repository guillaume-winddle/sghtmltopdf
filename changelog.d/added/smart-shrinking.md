`--enable-smart-shrinking` (`smart_shrinking` in the engine options) shrinks a document whose
content is wider than the page area to fit the paper, as wkhtmltopdf's smart shrinking does: the
document is laid out again on a page widened to its content, and drawn at the scale that keeps the
paper size. Without it, content wider than the page is cut at the paper's edge. Not available with
`--streaming`.
