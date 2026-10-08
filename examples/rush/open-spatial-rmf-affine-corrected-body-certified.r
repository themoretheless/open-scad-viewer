// @rush/1
// Original open spatial RMF with independently certified corrected filled caps.
profile = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 0.05mm)
path = bezier_curve(points: [[0,0,0],[0,0,1mm],[1mm,0,2mm],[0,1mm,3mm],[0,0,4mm],[0,0,5mm]])
show brep_progressive_sweep([[profile]],path,
 cap_correction_tolerance: 0.000000001mm,cap_correction_quantum: 0.0000000000009094947017729282mm,cap_correction_max_work: 1000000,
 scale: {degree: 1,knots: [0,0,1,1],values: [1.1,1.1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [7.16197243913529deg,7.16197243913529deg],weights: [1,1]},
 axis_scale: {degree: 1,knots: [0,0,1,1],values: [[1,1.25,0.75],[1,1.25,0.75]],weights: [1,1]},
 center_law: {degree: 1,knots: [0,0,1,1],values: [[0.01mm,-0.02mm,0.03mm],[0.01mm,-0.02mm,0.03mm]],weights: [1,1]},
 normal: [1,0,0],orientation: "rmf",rmf_transport_steps: 4096,error_max_cells: 100000,error_max_products: 1000000,spacing: "arc_length",initial_sections: 129,max_sections: 129,length_tolerance: 0.0001mm,length_max_cells: 100000,max_deviation: 0.25mm
).brep_tessellate(segments: 4)
