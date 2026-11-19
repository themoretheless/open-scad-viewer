"""Independent unit-density mass properties of the reflected elliptic cylinder."""
import math
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
reader = STEPControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
assert reader.TransferRoots() == 1
props = GProp_GProps()
BRepGProp.VolumeProperties_s(reader.OneShape(), props, 1e-11, True)
mass = 288*math.pi
assert math.isclose(props.Mass(), mass, rel_tol=1e-9)
assert max(abs(v-w) for v,w in zip(props.CentreOfMass().Coord(),(7,11,7))) < 1e-8
matrix = props.MatrixOfInertia()
for i, factor in enumerate([21,16,13],1):
    for j in range(1,4):
        assert abs(matrix.Value(i,j)-(mass*factor if i==j else 0)) < 1e-4
print(props.Mass(), props.CentreOfMass().Coord(), [matrix.Value(i,i) for i in range(1,4)])
