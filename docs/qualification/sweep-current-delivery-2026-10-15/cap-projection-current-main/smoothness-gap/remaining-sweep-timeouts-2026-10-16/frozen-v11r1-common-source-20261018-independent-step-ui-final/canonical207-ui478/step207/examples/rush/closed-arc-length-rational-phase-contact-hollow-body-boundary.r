// @rush/1
// Closed contact-fit outer/inner contours; complete boundary and actual material proof are separate.
outer = circle_curve(center: [3mm,0,0],normal: [0,1,0],radius: 0.25mm)
hole = circle_curve(center: [3mm,0,0],normal: [0,-1,0],radius: 0.125mm)
path = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 3mm)
rail = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 3.25mm)
show brep_progressive_sweep([[outer],[hole]],path,
 orientation: "rmf",normal: [0,0,1],orientation_guide: rail,contact_profile: 0,contact_parameter: 0,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 spacing: "arc_length",length_tolerance: 0.001mm,length_max_cells: 100000,
 initial_sections: 17,max_sections: 65,max_deviation: 2mm
).brep_tessellate(4)
