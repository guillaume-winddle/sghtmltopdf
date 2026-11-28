In a `table-layout: fixed` table whose columns all have a width and add up to less than the table,
the extra width is now spread over the columns in proportion to their widths (CSS 2.1 §17.5.2.1).
It used to be left empty after the last column, so three `width: 33%` columns did not reach the
table's right edge.
