// @rush/1
gear = brep_gear(module:1mm,teeth:20,height:2mm,pressure_angle:20,clearance:0.25mm,backlash:0.03mm)
mate = brep_gear(module:1mm,teeth:80,height:2mm,pressure_angle:20,internal:true,clearance:0.25mm,backlash:0.02mm)
result = gear.internal_spur_backlash(target_geometry:mate,centre_mm:30mm)
show result.brep_tessellate(3)
