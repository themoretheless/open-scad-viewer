# Surface diagnostic validation and retry, 2026-10-02

Surface distance and sampled boundary inspection now offer localized retry actions after current worker failure. A new calculation clears stale reports and retry state; existing cleanup invalidates older replies and cancels work. Boundary transport exceptions no longer show raw worker text.

Boundary numeric input is validated before dispatch: gap tolerance at least 0.000001 mm, angle 0..90 degrees, integer sample count 2..257. Each field has a localized accessible name, invalid state and associated correction message. Invalid values clear old boundary lines and prevent worker dispatch.

Eight focused UI tests pass, including existing boundary result/cancellation cases and new English/Russian boundary failure/retry/input cases and surface distance failure/retry cases. Tests verify raw errors are hidden, retry restores reports, invalid input does not dispatch, and documents remain identical. Typecheck and Vite pass. Dist measures 386,547 DirectModeler bytes and 7,202,651 total asset bytes; budget growth tracks measured UI additions.

These changes are unit-qualified; new browser mouse/keyboard and visual checks remain pending. Sampled boundary inspection still does not certify continuity over the whole interval or general G1/G2. No full P0–P3 completion or push/RAG publication is claimed.
