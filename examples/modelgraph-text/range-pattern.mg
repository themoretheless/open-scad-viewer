// @modelgraph-text/1
param count = 18 range 1..24
param orbit = 20mm range 5mm..50mm
param height = 4mm range 1mm..10mm

angles = 0deg..<360deg count count
part = box([2mm, 2mm, height])
parts = [
  for angle in angles
  => part.translate(x: orbit).rotate(z: angle)
]
show parts

assert parts. hasBodies(count).isWatertight()
assert measure(parts).height. approximately(height, tolerance: 0.01mm)
