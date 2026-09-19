// @modelgraph-text/1
segments 24
param columns: 4 range 1..8
param rows: 3 range 1..6

points = (0..<columns)
  .selectMany(x => (0..<rows).select(y => {x: x, y: y, size: 1 + (x+y)%3}))
  .orderBy(p => p.size)
  .thenBy(p => p.x)

parts = foreach p in points
  size = p.size
  yield box([size,size,size]).translate([p.x*5,p.y*5,0])

show parts
