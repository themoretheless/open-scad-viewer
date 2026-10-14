// @rush/1
// Discrete rotation-minimizing section sweep, with a sampled refinement diagnostic.
// The budget is not a certified distance to the continuous ideal sweep.
profile = bezier_curve(points: [[0mm,0mm,0mm],[3mm,0mm,0mm]])
path = bezier_curve(points: [[0mm,0mm,0mm],[0mm,12mm,8mm],[12mm,20mm,16mm]])
show framed_sweep(profile,path,normal: [1,0,0],sections: 16,max_deviation: 0.25mm).tessellate(segments_u: 16,segments_v: 32)
