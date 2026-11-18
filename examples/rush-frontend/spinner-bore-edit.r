// @rush/1
param diameter = 44mm range 43mm..44mm
sun = gear(teeth: 32, module: 1.5mm, thickness: 10mm, helix_angle: -35deg,
  herringbone: true, bore: 42mm, clearance: 0.1mm, backlash: 0.05mm)
show sun.subtract(cylinder(diameter / 2, 10mm))
