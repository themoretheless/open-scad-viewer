# Face and shell distance controls, 2026-10-02

## Verification

Four actual browser scenarios passed: face and shell distances using mouse and sequential keyboard controls. Chrome Canary 157.0.8081.0 / native Apple Metal remains active; strict console/page error gates are preserved. Keyboard controls use Tab, Enter, typed numeric input and native select type-ahead character events, with no programmatic fill/click/selectOption in keyboard mode. Fixture uploads use setInputFiles in both modes. Face mode uses 658 Tab presses and shell mode 1123.

The plate with hole/probe fixture has analytical distance 2.1360009363293826 mm. Face interval [2.13600023408,2.13691566721] and shell interval [2.13600023408,2.1360015742] contain it and meet 0.001 mm tolerance. Both face witnesses are rendered. The nested spheres fixture gives shell interval [3,3] mm and explicitly reports that volume nesting/intersection is not classified.

Invalid face 9999 and same-body shell target show localized errors and clear previous witnesses. Deliberately held calculations are terminated with Escape; reopening produces fresh native replies. Complete downloaded documents remain identical after measurement and reload. Instrumentation captures native results before reload resets counters: six face requests, five shell requests. Final keyboard face and nested-shell screenshots were actually inspected.

## Changes and limits

Face qualification previously accepted --keyboard but still clicked and filled controls. It now exercises real keyboard actions. Both scripts admit an explicit native browser executable, require active GPU when requested, check all console errors, and qualify exact document reload. No geometry/product changes or kernel rebuild were made in this block.

Evidence covers two controlled fixtures, not arbitrary B-rep geometry, all self-intersections, worker crash retry, every diagnostic command, or the whole P0–P3 plan. Native select type-ahead behavior is qualified on this browser/backend only. No push or RAG upload is claimed.
