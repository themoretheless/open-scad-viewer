# WASM → worker → redesigned UI qualification

2026-10-08. Four unequal-radius annular control bodies from the original
2026-10-07 qualification fixtures passed through the rebuilt native WASM,
real browser worker and redesigned Solid workspace. Each reports 31/31 faces,
465/465 pairs, no unresolved or unvisited work, and absenceProven=true.

## Verified behavior

- Real intersecting cube faces produce a visible red contact marker; the UI
  retains the incomplete diagnostic status for that defective model.
- An injected worker failure displays a localized error; keyboard Retry succeeds.
- Esc cancels a pending worker operation. An obsolete response delivered while
  a new operation is pending cannot replace the new result or clear its state.
- Button cancellation and a fresh real retry also succeed.
- Exported documents before and after each diagnosis are identical.
- Browser page errors: zero. Request-to-response time for the annular checks
  and retries: 1.69–2.03 seconds, including worker/module startup.

The screenshot uses the CPU/SVG fallback in headless Chromium. GPU rendering
was not qualified. This result covers these four control bodies, not arbitrary
NURBS fillets, all thickness cases, G1/G2 certification, or independent STEP
acceptance.

## Validation

- 414 tests passed across seven transport, worker and UI test files.
- 13 native boundary-hull regression tests passed.
- Source and MCP TypeScript checks passed.
- Full npm build and distribution verification passed: 152 artifacts,
  8,327,378 asset bytes, 15,251,526 raw WASM bytes.
- Geometry WASM: 11,955,134 bytes; SHA256
  `0adc6b44273109e5a7b65d8197b3722509c6cfe77c5519ad5bc6d239be595b0b`.

The measured bundle limits were updated while retaining their existing margins:
geometry chunk 3,988,192 bytes (actual 3,987,584), Solid worker 167,748
(actual 167,625), total assets 8,328,139 (actual 8,327,378).

## Reproduce

```sh
npm run build
npx tsx scripts/prepare-solid-self-intersection-fixtures.mts
node scripts/check-solid-self-intersection-browser.mjs
```

See the script arguments for fixture and report directories. Native models and
certificates remain owned by Rust; TypeScript validates transport metadata and
binds certificates to their original surfaces and source frames.

`result.json` records the successful browser run; `document-identity.json`
records before/after hashes. Compressed logs preserve final successful checks.
`contact-highlight.png` shows the real defect marker in the redesigned UI.
