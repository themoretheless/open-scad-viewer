// @rush/1
// Open spatial RMF with simultaneous rational laws and corrected annular caps.
profile = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 0.05mm)
hole = circle_curve(center: [0,0,0],normal: [0,0,-1],radius: 0.02mm)
path = bezier_curve(points: [[0,0,0],[0,0,1mm],[1mm,0,2mm],[0,1mm,3mm],[0,0,4mm],[0,0,5mm]])
show brep_progressive_sweep([[profile],[hole]],path,
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,
 scale: {degree: 2,knots: [0,0,0,1,1,1],values: [1.1,1.2,1.1],weights: [1,2,1]},
 twist: {degree: 2,knots: [0,0,0,1,1,1],values: [7.16197243913529deg,11.459155902616464deg,7.16197243913529deg],weights: [1,2,1]},
 axis_scale: {degree: 2,knots: [0,0,0,1,1,1],values: [[1,1.25,0.75],[1.2,1.1,0.8],[1,1.25,0.75]],weights: [1,2,1]},
 center_law: {degree: 2,knots: [0,0,0,1,1,1],values: [[0.01mm,-0.02mm,0.03mm],[0.02mm,0,0.01mm],[0.01mm,-0.02mm,0.03mm]],weights: [1,2,1]},
 normal: [1,0,0],orientation: "rmf",rmf_transport_steps: 4096,error_max_cells: 100000,error_max_products: 1000000,spacing: "parameter",initial_sections: 127,max_sections: 127,length_tolerance: 0.0001mm,length_max_cells: 100000,max_deviation: 0.25mm
).brep_tessellate(segments: 2)
