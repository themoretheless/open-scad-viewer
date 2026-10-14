// @rush/1
// Exact oblique source plane. Opposite endpoint tangents with two stations.
// Original planar corrected-frame error is bounded; this surface preview is not a Solid.
profile = bezier_curve(points: [[1mm,1mm,1mm],[2mm,2mm,2mm]])
path = bezier_curve(points: [[0,0,0],[1mm,-1mm,0],[2mm,0,-2mm],[2mm,-2mm,0],[1mm,-1mm,0]])
show progressive_sweep(profile,path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [1,1,1],orientation: "corrected_frenet",initial_sections: 2,max_sections: 17,max_deviation: 10mm
).nurbs_patches_tessellate(4)
