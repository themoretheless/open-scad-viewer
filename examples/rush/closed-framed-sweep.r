// @rush/1
// Closed discrete frame sweep with a C0 seam; sampled budget is not a continuous certificate.
profile = bezier_curve(points: [[12mm,0mm,0mm],[15mm,0mm,0mm]])
path = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 12mm)
show framed_sweep(profile,path,normal: [1,0,0],sections: 16,max_deviation: 0.6mm).tessellate(segments_u: 16,segments_v: 32)
