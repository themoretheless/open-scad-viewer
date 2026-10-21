// @rush/1
// Certified boundary error does not prove global embedding of a Fixed-frame sweep.
profile = circle_curve(center: [0,0,0],normal: [0,1,0],radius: 0.25mm)
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 10mm)
show brep_progressive_sweep([[profile]],path,
 orientation: "fixed",normal: [0,0,1],spacing: "arc_length",
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 initial_sections: 5,max_sections: 129,length_tolerance: 0.001mm,max_deviation: 0.02mm
).brep_tessellate(4)
