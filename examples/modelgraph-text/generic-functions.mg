// @modelgraph-text/1
// Generic records, column parameters, named results and destructuring.
struct Point3D<T> {
    x: f32,
    y: f32,
    z: T,
}

makePoint = fn<T>
    x: f32,
    y: f32,
    z: T,
->
    point: Point3D<T>,
    count: int,
{
    ret {
        point: Point3D<T> { x, y, z },
        count: 1,
    }
}

param height = 12 range 1..40
{ point, count } = makePoint(10, 20, height)
body = box([point.x, point.y, point.z])
show body
assert body. hasBodies(count)
