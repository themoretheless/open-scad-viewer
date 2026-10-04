# Profile preparation refusal and retry matrix

Six new UI regression scenarios cover three worker refusal messages in both English and Russian: unclamped NURBS endpoints, unequal sketch planes, and unsupported preparation inputs. Each scenario injects a worker refusal, checks the translated next action and absence of the original private message, preserves the complete document, retries identical worker arguments, resolves through the real native profile preparation implementation, applies with Enter and restores the complete input document with Undo. The first input identity is retained and the second chain is consumed only on Apply.

The test renderer simulates component events and worker refusals. These scenarios establish UI refusal/retry behavior, not native browser keyboard/mouse qualification or native generation of the three injected failures. Existing separate tests cover real gap and crossing reports, Escape and superseded tolerance replies. Full 95-command browser execution coverage remains incomplete.

Focused check: six passed. Full UI regression: 310 passed, 61.75 seconds. Diff whitespace check passes. Product source and production artifacts are unchanged in this qualification increment.

## Localized input context follow-up

The UI now also translates duplicate-input and noncoplanar-control-point refusals. The latter states the actual 1e-7 mm plane-deviation limit. Refusal messages include the selected input names. Ten injected refusal/retry scenarios pass in English/Russian; full UI regression passes 314 tests in 66.11 seconds. TypeScript, Vite production build and whitespace checks pass. Dist verification passes with 7,218,858 asset bytes and 400,560 DirectModeler bytes; the added messages cost 814 bytes. Artifact budgets increase by exactly that amount, retaining the previous margins. No browser qualification of the new messages was performed in this increment.
