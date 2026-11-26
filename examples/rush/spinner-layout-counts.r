// @rush/1
gear = brep_gear(module:1mm,teeth:4,height:2mm,pressure_angle:20,clearance:0.25mm,backlash:0.03mm)
result = gear.spinner_layout_counts(sun_teeth:32,minimum_gap_mm:0mm)
show result.brep_tessellate(3)
