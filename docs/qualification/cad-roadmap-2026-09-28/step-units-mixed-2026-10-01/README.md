# STEP units and mixed assemblies — 2026-10-01

Current source at 260c0e9e, unchanged geometry WASM b2ebc7ddc537db72d15b0449123d506cfa75a15cd47e788b07c38a3f6146eb06. Both exporters and independent OpenCascade verification completed with exit 0. Python 3.12.14, cadquery-ocp 8.0.1.0.0.

Units: nine import → +1 mm top-face Push/Pull → current STEP export cycles for bracket, enclosure and flange with centimetre, metre and inch units. All 18 input/output files independently read, valid, one solid, dimensions and volume match their manifest. Max bounds error 1.000043994281441e-7 mm; maximum relative volume error 9.301730032423822e-12.

Mixed assembly: millimetre and inch components, with and without distinct source/target placements. Four input/output files independently read, valid, two solids, dimensions and volume match. Source importer preserves edited body ID, untouched component topology IDs and vertices and original STEP. Max bounds error 1.0000002248489182e-7 mm; relative volume error 1.1724054992009035e-15.

Both OCCT preview images were visually inspected. Fixture sources are authored by this project. These results qualify these fixtures, not arbitrary external STEP topology, all curved face edits or full assembly metadata exchange. Full roadmap remains open.

Reproduce with node --import tsx scripts/export-cad-step-unit-cycles.mts OUTPUT and scripts/export-cad-step-mixed-assembly.mts OUTPUT, then /private/tmp/cad-roadmap-ocp/bin/python scripts/verify-cad-step-occt.py OUTPUT. Manifests retain each exact STEP hash and independently specified expected measurements.
