// @rush/1
// Original closed planar RMF frame; retained boundary and material are audited separately.
profile = circle_curve(center: [4mm,0,0],normal: [0,1,0],radius: 0.2mm)
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 4mm)
show brep_progressive_sweep([[profile]],path,
 orientation: "rmf",normal: [0,0,1],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 spacing: "arc_length",length_tolerance: 0.001mm,length_max_cells: 100000,
 initial_sections: 17,max_sections: 65,max_deviation: 2mm
).brep_tessellate(4)
