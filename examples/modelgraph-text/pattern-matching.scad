// @modelgraph-text/1
segments 40

items = [
  {kind: "box", size: [12,16,8], position: [0,0,0]},
  {kind: "ball", radius: 4, position: [24,8,4]},
  {kind: "combined", radii: [2,1,3], position: [40,8,6]},
  {kind: "hidden"}]

parts = foreach item in items
  yield match item
    {kind: "box", size: [w,d,h], position} where w > 0 && d > 0 && h > 0 =>
      box([w,d,h]).translate(position)
    {kind: "sphere" | "ball", radius: r @ 1..10, position} =>
      sphere(r).translate(position)
    {kind: "combined", radii: [first,..rest], position} =>
      radius = first + rest.sum()
      ret sphere(radius).translate(position)
    _ => []

show parts
