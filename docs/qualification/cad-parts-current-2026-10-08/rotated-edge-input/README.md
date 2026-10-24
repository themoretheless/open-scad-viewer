# Rotated cuboid single-edge input

Variable radius passes both mouse and keyboard scenarios: local oversize error,
readable dimension field, zoom/resize, invalid quantity refusal and recovery,
real cancel/late replies, apply without recomputing, Undo/Redo and exact
reload. The fixture retains the original rotated cuboid and selects one edge.

Constant radius on that edge remains refused at 0.1, 1 and 2 mm by the current
kernel: “Fillet boundary collides with a nonadjacent span or separation is unresolved”.
This compound refusal does not prove a collision or an oversized radius.
The UI now says that intersection freedom could not be confirmed; the selected
edge is highlighted, commit is blocked, Retry remains refused, and cancel/reload
preserve the original document. Both mouse and keyboard refusal scenarios pass.
The geometric cause and successful constant operation remain open.

The native diagnostic binds the original fixture and kernel; receipt hashes bind
all reports. These runs use kernel 8f28d264 and do not qualify the new factored
normal package. Variable cases were repeated on the same current UI and script as the refusal
cases. Successful general NURBS fillets are not established.

## Native correction after this browser receipt

Commit `d39aab137` fixes a false coincidence admission for parallel curves on different supporting lines. All twelve cuboid edges at radii 0.1, 1 and 2 mm pass natively before and after rigid placement, with closed boundaries and independent analytic volumes. Evidence: `../constant-radius-support-lines/`. The refusal recordings above describe the prior packaged kernel; the corrected WASM and browser operation still require qualification.
