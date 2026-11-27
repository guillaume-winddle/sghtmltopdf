A table with `width: auto` is now as wide as its columns, up to the containing width, as in
browsers (CSS 2.1 §17.5.2.2). It used to fill the containing width, so a small key/value table
was spread across the page; `margin: auto` now centres it. The width a table measures for
shrink-to-fit (floats, inline-blocks, cells) now sums the widest cell of each column, adds
`border-spacing`, and is no narrower than the caption. A cell `width` in px is its column's
max-content width, and a `<col>` width in px counts in the table's width.
