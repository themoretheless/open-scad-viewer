// @modelgraph-text/1
param height = 12 range 1..40

fn dimensions[T] value: T -> x1: T, x2: int, x3: int, x4: int
    ret
        x1: value, x2: 10,
        x3: 20, x4: 1

fn makeBox
    width: int,
    depth: int,
    height: f64
-> Geometry
    size = [width, depth, height]
    ret box(size)

r = dimensions(height)
show makeBox(r.x2, r.x3, r.x1)
