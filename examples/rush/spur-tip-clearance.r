// @rush/1
gear = brep_gear(module:1mm,teeth:20,height:2mm,pressure_angle:20,clearance:0.25mm,backlash:0.03mm)
mate = brep_gear(module:1mm,teeth:40,height:2mm,pressure_angle:20,internal:false,clearance:0.25mm,backlash:0.02mm)
result = gear.spur_tip_clearance(target_geometry:mate,centre_mm:30mm)
show result.brep_tessellate(3)
