// @rush/1
gear = planetary_gear_set_step(module:1.5mm,teeth:4,sun_teeth:32,height:10mm,pressure_angle:20,helix_angle:0,herringbone:false,clearance:0.1mm,backlash:0.05mm,orbit_angles:[0,20,40,60,80,100,120,140,160,180,200,220,240,260,280,300,320,340],index:9)
show gear.brep_tessellate(3)
