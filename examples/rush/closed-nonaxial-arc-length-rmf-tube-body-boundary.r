// @rush/1
// Original closed ellipse in exact nonaxial plane x+y=0; Solid admission is separate.
profile = circle_curve(center: [4mm,-4mm,0],normal: [0,0,1],radius: 0.2mm)
path = nurbs_curve(degree: 2,knots: [0,0,0,0.25,0.25,0.5,0.5,0.75,0.75,1,1,1],control_points: [[4mm,-4mm,0],[4mm,-4mm,4mm],[0,0,4mm],[-4mm,4mm,4mm],[-4mm,4mm,0],[-4mm,4mm,-4mm],[0,0,-4mm],[4mm,-4mm,-4mm],[4mm,-4mm,0]],weights: [1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1,0.7071067811865476,1])
show brep_progressive_sweep([[profile]],path,
 orientation: "rmf",normal: [1,1,0],
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 spacing: "arc_length",length_tolerance: 0.001mm,length_max_cells: 100000,
 initial_sections: 17,max_sections: 65,max_deviation: 2mm
).brep_tessellate(4)
