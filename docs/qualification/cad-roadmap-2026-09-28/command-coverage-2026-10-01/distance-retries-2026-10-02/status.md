# Distance worker failure retry, 2026-10-02

Edge, face and shell distance panels now expose separate localized retry buttons after a current worker failure. Each retry invalidates/cancels the preceding request and runs the same chosen geometry and budget through the existing worker. Changed input, closed panels and fresh results clear retry state. Validation errors do not expose a misleading retry button.

Six actual native Chrome Canary 157 / Apple Metal scenarios passed: each panel by mouse and sequential keyboard. Each injects a one-shot CAD_TRANSPORT error envelope, verifies private worker text is absent and old witnesses are cleared, activates the appropriate retry and waits for a fresh native tolerance result. Existing invalid-choice, held-request Escape termination, exact-document comparison and reload checks pass. Counters include the failed/retried requests: edge 6, face 8, shell 7. Strict console/page errors and GPU-active checks remain enabled.

All 288 DirectModeler UI tests pass. Typecheck, Vite and dist pass. DirectModeler measures 384,040 bytes and all assets 7,200,144 bytes, within current budgets. No kernel rebuild or geometry algorithm change was required. Controlled fixtures remain the cylinder, trimmed plate/probe and nested spheres; they do not certify general B-rep geometry or complete P0–P3.
