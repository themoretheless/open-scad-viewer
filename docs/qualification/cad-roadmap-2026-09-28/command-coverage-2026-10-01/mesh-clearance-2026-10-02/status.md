# Mesh clearance failure recovery, 2026-10-02

Current worker failure shows a localized actionable message and Retry mesh clearance button. Retry reuses selected meshes, clears previous reports and uses the existing independent worker owner. Changed input or a closed panel invalidates the old reply and clears retry state. Invalid same-body selection remains an input error.

Five focused tests pass, including both English/Russian transport failure and successful retry cases, exact document preservation, normal witness display, small overlap and stale target-change reply behavior. Typecheck, Vite and dist verification pass. DirectModeler measures 384,502 bytes and all assets 7,200,606 bytes; budgets increased only for measured UI growth.

Actual mouse and sequential keyboard browser scenarios passed on Chrome Canary 157 / Apple Metal with GPU active and strict console/page error gates. A one-shot transport failure clears witnesses and hides private details. Retry restores 3 mm gap between two closed cube meshes; both witness endpoints appear. Whole documents remain equal after retry and reload. Keyboard mode used 556 Tab presses. Failure and recovery screenshots were visually inspected.

This is mesh-based clearance on controlled cubes, not general exact B-rep distance certification. No general P0–P3 completion or publication is claimed.
