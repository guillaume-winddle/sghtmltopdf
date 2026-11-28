An absolutely positioned box with both `top` and `bottom` and `height: auto` now takes the height of
its containing block between them (CSS 2.1 §10.6.4), when that height is known. It used to be only
as tall as its content, so an empty one (a line drawn by a border) had no height and was not drawn.
