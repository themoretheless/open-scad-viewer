#!/usr/bin/env python3
"""Independently verify the rational hollow STEP fixtures with OCCT/OCP.

Checks STEP transfer, unique topology (TopoDS_Shape.IsSame), closed 2-manifold
incidence, BRep validity, exact B-rep mass properties, optimal bounds, and
sampled planar sections. This script does not import or execute the exporter.
"""

import argparse
import hashlib
import json
import math
import platform
from datetime import datetime, timezone
from pathlib import Path

import OCP
from OCP.Bnd import Bnd_Box
from OCP.BRep import BRep_Tool
from OCP.BRepAdaptor import BRepAdaptor_Curve, BRepAdaptor_Surface
from OCP.BRepAlgoAPI import BRepAlgoAPI_Section
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.gp import gp_Dir, gp_Pln, gp_Pnt
from OCP.IFSelect import IFSelect_RetDone
from OCP.Precision import Precision
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_EDGE, TopAbs_FACE, TopAbs_SHELL, TopAbs_SOLID, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS


EXPECTED = {
    "solids": 1,
    "faces": 10,
    "uniqueEdges": 24,
    "uniqueVertices": 16,
    "volume": 560.0 * math.pi / 3.0,
    "area": math.pi * (9.0 * math.sqrt(109.0) + 3.0 * math.sqrt(101.0) + 40.0),
    "bounds": [-6.0, 6.0, -6.0, 6.0, 0.0, 10.0],
    "sections": [
        {"z": 0.0, "innerRadius": 1.0, "outerRadius": 3.0},
        {"z": 5.0, "innerRadius": 1.5, "outerRadius": 4.5},
        {"z": 10.0, "innerRadius": 2.0, "outerRadius": 6.0},
    ],
}
TOLERANCES = {
    "volumeAbsolute": 1.0e-7,
    "areaAbsolute": 1.0e-7,
    "boundsAbsolute": 1.0e-7,
    "sectionRadiusAbsolute": 1.0e-7,
    "sectionAxialAbsolute": 1.0e-7,
}
SECTION_SAMPLES_PER_EDGE = 65


def unique_subshapes(shape, kind):
    """Return unique subshapes using IsSame, not Python or hash identity."""
    unique = []
    explorer = TopExp_Explorer(shape, kind)
    while explorer.More():
        candidate = explorer.Current()
        if not any(candidate.IsSame(existing) for existing in unique):
            unique.append(candidate)
        explorer.Next()
    return unique


def topology(shape):
    faces = unique_subshapes(shape, TopAbs_FACE)
    edges = unique_subshapes(shape, TopAbs_EDGE)
    vertices = unique_subshapes(shape, TopAbs_VERTEX)
    solids = unique_subshapes(shape, TopAbs_SOLID)
    shells = unique_subshapes(shape, TopAbs_SHELL)

    # Count every edge occurrence in every face. A seam therefore contributes
    # two incidences even when both occurrences belong to the same face.
    incidence = [[] for _ in edges]
    orientation_incidence = [[] for _ in edges]
    for face_index, face in enumerate(faces):
        explorer = TopExp_Explorer(face, TopAbs_EDGE)
        while explorer.More():
            edge_occurrence = explorer.Current()
            for edge_index, edge in enumerate(edges):
                if edge_occurrence.IsSame(edge):
                    incidence[edge_index].append(face_index)
                    orientation_incidence[edge_index].append(str(edge_occurrence.Orientation()))
                    break
            else:
                raise RuntimeError("A face edge was not found in the unique edge list")
            explorer.Next()

    edge_reports = [
        {
            "edge": index,
            "faceOccurrences": faces_for_edge,
            "faceOccurrenceCount": len(faces_for_edge),
            "orientations": orientation_incidence[index],
        }
        for index, faces_for_edge in enumerate(incidence)
    ]
    bad_edges = [entry for entry in edge_reports if entry["faceOccurrenceCount"] != 2]
    shell_closed = [BRep_Tool.IsClosed_s(shell) for shell in shells]
    euler_characteristic = len(vertices) - len(edges) + len(faces)
    return {
        "counts": {
            "solids": len(solids),
            "shells": len(shells),
            "faces": len(faces),
            "uniqueEdges": len(edges),
            "uniqueVertices": len(vertices),
            "faceEdgeOccurrences": sum(len(items) for items in incidence),
        },
        "identityComparison": "TopoDS_Shape.IsSame",
        "shellClosed": shell_closed,
        "closed2Manifold": bool(shells) and all(shell_closed) and not bad_edges,
        "edgesWithoutExactlyTwoFaceOccurrences": bad_edges,
        "eulerCharacteristicVEPlusF": euler_characteristic,
    }


def mass_properties(shape):
    volume = GProp_GProps()
    area = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, volume, True, False, False)
    BRepGProp.SurfaceProperties_s(shape, area, False, False)
    return volume.Mass(), area.Mass()


def optimal_bounds(shape):
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    if box.IsVoid():
        raise RuntimeError("Optimal bounding box is void")
    # OCP 8.0.1.0 exposes Bnd_Box.Get() as unregistered Bnd_Box::Limits;
    # the corner accessors return the same six limits without that binding issue.
    lower = box.CornerMin()
    upper = box.CornerMax()
    return [lower.X(), upper.X(), lower.Y(), upper.Y(), lower.Z(), upper.Z()]


def sampled_section(shape, z, samples_per_edge=SECTION_SAMPLES_PER_EDGE):
    plane = gp_Pln(gp_Pnt(0.0, 0.0, z), gp_Dir(0.0, 0.0, 1.0))
    operation = BRepAlgoAPI_Section(shape, plane, True)
    operation.ComputePCurveOn1(False)
    operation.ComputePCurveOn2(False)
    operation.Build()
    if not operation.IsDone():
        raise RuntimeError(f"Planar section failed at z={z}")
    section_shape = operation.Shape()
    edges = unique_subshapes(section_shape, TopAbs_EDGE)
    if not edges:
        raise RuntimeError(f"Planar section has no edges at z={z}")

    edge_reports = []
    edge_sample_radii = []
    all_samples = []
    for edge_index, edge in enumerate(edges):
        curve = BRepAdaptor_Curve(TopoDS.Edge(edge))
        first = curve.FirstParameter()
        last = curve.LastParameter()
        samples = []
        for sample_index in range(samples_per_edge):
            parameter = first + (last - first) * sample_index / (samples_per_edge - 1)
            point = curve.Value(parameter)
            sample = {
                "x": point.X(),
                "y": point.Y(),
                "z": point.Z(),
                "radius": math.hypot(point.X(), point.Y()),
            }
            samples.append(sample)
            all_samples.append(sample)
        radii = [sample["radius"] for sample in samples]
        edge_sample_radii.append(radii)
        edge_reports.append({
            "edge": edge_index,
            "samples": len(samples),
            "minimumRadius": min(radii),
            "maximumRadius": max(radii),
            "meanRadius": math.fsum(radii) / len(radii),
            "maximumAbsZError": max(abs(sample["z"] - z) for sample in samples),
        })

    # Every section edge is expected to belong to one of two radial bands.
    # Determine the bands solely from the measured edge mean radii.
    inner_band = min(edge_reports, key=lambda entry: entry["meanRadius"])["meanRadius"]
    outer_band = max(edge_reports, key=lambda entry: entry["meanRadius"])["meanRadius"]
    threshold = (inner_band + outer_band) / 2.0
    inner_radii = []
    outer_radii = []
    for entry, radii in zip(edge_reports, edge_sample_radii):
        target = inner_radii if entry["meanRadius"] < threshold else outer_radii
        target.extend(radii)

    return {
        "z": z,
        "sectionEdgeCount": len(edges),
        "samplesPerEdge": samples_per_edge,
        "sampleCount": len(all_samples),
        "measuredInnerRadius": math.fsum(inner_radii) / len(inner_radii),
        "measuredOuterRadius": math.fsum(outer_radii) / len(outer_radii),
        "sampledInnerRadiusRange": [min(inner_radii), max(inner_radii)],
        "sampledOuterRadiusRange": [min(outer_radii), max(outer_radii)],
        "maximumAbsZError": max(abs(sample["z"] - z) for sample in all_samples),
        "edges": edge_reports,
    }


def sampled_surface_section(shape, z, samples_per_face=SECTION_SAMPLES_PER_EDGE):
    """Sample exact stored surfaces at constant z, without section-curve fitting."""
    face_reports = []
    face_sample_radii = []
    all_samples = []
    for face_index, face in enumerate(unique_subshapes(shape, TopAbs_FACE)):
        surface = BRepAdaptor_Surface(TopoDS.Face(face))
        u_first = surface.FirstUParameter()
        u_last = surface.LastUParameter()
        v_first = surface.FirstVParameter()
        v_last = surface.LastVParameter()
        u_middle = (u_first + u_last) / 2.0
        z_first = surface.Value(u_middle, v_first).Z()
        z_last = surface.Value(u_middle, v_last).Z()
        if abs(z_last - z_first) <= 1.0e-12:  # planar cap, not a lateral face
            continue
        if not min(z_first, z_last) - 1.0e-12 <= z <= max(z_first, z_last) + 1.0e-12:
            continue

        radii = []
        max_z_error = 0.0
        for sample_index in range(samples_per_face):
            u = u_first + (u_last - u_first) * sample_index / (samples_per_face - 1)
            low = v_first
            high = v_last
            z_low = surface.Value(u, low).Z()
            z_high = surface.Value(u, high).Z()
            if abs(z_low - z) <= 1.0e-14:
                v = low
            elif abs(z_high - z) <= 1.0e-14:
                v = high
            else:
                if not min(z_low, z_high) <= z <= max(z_low, z_high):
                    raise RuntimeError(f"z={z} is outside a lateral surface at u={u}")
                for _ in range(80):
                    middle = (low + high) / 2.0
                    z_middle = surface.Value(u, middle).Z()
                    if (z_low <= z <= z_middle) or (z_middle <= z <= z_low):
                        high = middle
                        z_high = z_middle
                    else:
                        low = middle
                        z_low = z_middle
                v = (low + high) / 2.0
            point = surface.Value(u, v)
            sample = {
                "z": point.Z(),
                "radius": math.hypot(point.X(), point.Y()),
            }
            all_samples.append(sample)
            radii.append(sample["radius"])
            max_z_error = max(max_z_error, abs(point.Z() - z))
        face_sample_radii.append(radii)
        face_reports.append({
            "face": face_index,
            "surfaceType": str(surface.GetType()),
            "samples": len(radii),
            "minimumRadius": min(radii),
            "maximumRadius": max(radii),
            "meanRadius": math.fsum(radii) / len(radii),
            "maximumAbsZError": max_z_error,
        })

    if not face_reports:
        raise RuntimeError(f"No stored lateral surfaces sampled at z={z}")
    inner_band = min(face_reports, key=lambda entry: entry["meanRadius"])["meanRadius"]
    outer_band = max(face_reports, key=lambda entry: entry["meanRadius"])["meanRadius"]
    threshold = (inner_band + outer_band) / 2.0
    inner_radii = []
    outer_radii = []
    for entry, radii in zip(face_reports, face_sample_radii):
        (inner_radii if entry["meanRadius"] < threshold else outer_radii).extend(radii)
    return {
        "z": z,
        "method": "Direct stored-surface evaluation; bisection in V at fixed U",
        "lateralFaceCount": len(face_reports),
        "samplesPerFace": samples_per_face,
        "sampleCount": len(all_samples),
        "measuredInnerRadius": math.fsum(inner_radii) / len(inner_radii),
        "measuredOuterRadius": math.fsum(outer_radii) / len(outer_radii),
        "sampledInnerRadiusRange": [min(inner_radii), max(inner_radii)],
        "sampledOuterRadiusRange": [min(outer_radii), max(outer_radii)],
        "maximumAbsZError": max(abs(sample["z"] - z) for sample in all_samples),
        "faces": face_reports,
    }


def verify_file(path):
    result = {
        "path": str(path.resolve()),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "sizeBytes": path.stat().st_size,
    }
    reader = STEPControl_Reader()
    status = reader.ReadFile(str(path))
    result["readStatus"] = str(status)
    result["readDone"] = status == IFSelect_RetDone
    if not result["readDone"]:
        result["passed"] = False
        result["error"] = "STEP ReadFile did not return IFSelect_RetDone"
        return result

    roots = reader.TransferRoots()
    result["transferredRoots"] = roots
    result["transferredShapes"] = reader.NbShapes()
    if roots == 0 or reader.NbShapes() == 0:
        result["passed"] = False
        result["error"] = "STEP transfer produced no shapes"
        return result
    shape = reader.OneShape()
    result["bRepCheckValid"] = BRepCheck_Analyzer(shape, True).IsValid()
    result["topology"] = topology(shape)
    result["volume"], result["area"] = mass_properties(shape)
    result["volumeError"] = result["volume"] - EXPECTED["volume"]
    result["areaError"] = result["area"] - EXPECTED["area"]
    result["bounds"] = optimal_bounds(shape)
    result["boundsErrors"] = [value - expected for value, expected in zip(result["bounds"], EXPECTED["bounds"])]
    result["occtConfusion"] = Precision.Confusion_s()
    result["sections"] = []
    result["planarSectionDiagnostics"] = []
    for expected in EXPECTED["sections"]:
        for section, destination in (
            (sampled_surface_section(shape, expected["z"]), result["sections"]),
            (sampled_section(shape, expected["z"]), result["planarSectionDiagnostics"]),
        ):
            section["innerRadiusError"] = section["measuredInnerRadius"] - expected["innerRadius"]
            section["outerRadiusError"] = section["measuredOuterRadius"] - expected["outerRadius"]
            section["maximumSampleInnerRadiusError"] = max(
                abs(value - expected["innerRadius"]) for value in section["sampledInnerRadiusRange"]
            )
            section["maximumSampleOuterRadiusError"] = max(
                abs(value - expected["outerRadius"]) for value in section["sampledOuterRadiusRange"]
            )
            section["passed"] = (
                section["maximumSampleInnerRadiusError"] <= TOLERANCES["sectionRadiusAbsolute"]
                and section["maximumSampleOuterRadiusError"] <= TOLERANCES["sectionRadiusAbsolute"]
                and section["maximumAbsZError"] <= TOLERANCES["sectionAxialAbsolute"]
            )
            destination.append(section)
        result["planarSectionDiagnostics"][-1]["diagnosticOnly"] = True

    counts = result["topology"]["counts"]
    checks = {
        "readDone": result["readDone"],
        "oneSolid": counts["solids"] == EXPECTED["solids"],
        "bRepCheckValid": result["bRepCheckValid"],
        "faceCount": counts["faces"] == EXPECTED["faces"],
        "uniqueEdgeCount": counts["uniqueEdges"] == EXPECTED["uniqueEdges"],
        "uniqueVertexCount": counts["uniqueVertices"] == EXPECTED["uniqueVertices"],
        "closed2Manifold": result["topology"]["closed2Manifold"],
        "volume": abs(result["volumeError"]) <= TOLERANCES["volumeAbsolute"],
        "area": abs(result["areaError"]) <= TOLERANCES["areaAbsolute"],
        # AddOptimal uses Precision::Confusion for numerical calculations even
        # with useShapeTolerance=False; allow only that documented padding.
        "bounds": all(abs(error) <= result["occtConfusion"] + 1.0e-12 for error in result["boundsErrors"]),
        "sections": all(section["passed"] for section in result["sections"]),
    }
    result["checks"] = checks
    result["passed"] = all(checks.values())
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path, help="STEP files to verify")
    parser.add_argument("--output", type=Path, help="JSON report path")
    args = parser.parse_args()
    results = [verify_file(path) for path in args.paths]
    report = {
        "schemaVersion": 1,
        "generatedAtUtc": datetime.now(timezone.utc).isoformat(),
        "environment": {
            "python": platform.python_version(),
            "ocp": OCP.__version__,
            "ocpModule": OCP.__file__,
        },
        "method": {
            "exporterImported": False,
            "sectionSamplesPerEdgeOrFace": SECTION_SAMPLES_PER_EDGE,
            "sections": "Direct stored-surface sampling is authoritative; BRepAlgoAPI_Section is retained as an approximation diagnostic",
            "bounds": "BRepBndLib.AddOptimal_s without triangulation or shape-tolerance enlargement; comparison permits documented Precision::Confusion padding",
            "manifold": "Every unique IsSame edge has two face occurrences and every shell is closed",
        },
        "ocpBindingCompatibility": [
            {
                "version": "8.0.1.0",
                "api": "Bnd_Box.Get()",
                "observed": "TypeError: Unregistered type: Bnd_Box::Limits",
                "probableLayer": "OCP Python binding return-value registration",
                "workaround": "CornerMin() and CornerMax()",
            },
            {
                "version": "8.0.1.0",
                "api": "TopoDS.Edge_s",
                "observed": "AttributeError; not exposed",
                "probableLayer": "OCP Python binding API spelling",
                "workaround": "TopoDS.Edge(shape)",
            },
        ],
        "expected": EXPECTED,
        "tolerances": TOLERANCES,
        "results": results,
        "passed": all(result["passed"] for result in results),
    }
    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text)
    print(text)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
