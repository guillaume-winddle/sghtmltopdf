A `position: relative` box that starts on a new page keeps its `top`/`bottom` offset. Pagination
took the offset for the box's place in the flow, so on every page after the first the box was
drawn at the top of the page and the boxes after it moved up by the offset.
