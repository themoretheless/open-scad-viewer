// @rush/1
// Negative sweep follows the opposite hand around the supplied normal.
show circle_arc(center: [0mm,0mm,0mm], normal: [1,2,3], radius: 10mm, start_degrees: 25deg, sweep_degrees: -220deg).surface_extrude([0mm,0mm,12mm]).tessellate(segments_u: 16, segments_v: 32)
