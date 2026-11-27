`vertical-align: middle` and `bottom` now apply in a table cell that has a `height`. The height made
the cell's box as tall as the row, so no space was left to distribute and the content stayed at the
top. A cell height is a minimum: the content is now aligned between itself and the cell's edges, as
in browsers.
