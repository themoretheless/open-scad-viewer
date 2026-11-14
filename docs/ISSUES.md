# Known Issues — 11 open of the original Top 500

> Updated 2026-11-13: removed 478 resolved items. Domains A–D (parser/geometry,
> renderer/math, App state/logic, UX/CSS/a11y) cited line numbers in
> `src/services/openscadParser.ts`, `src/services/webgpuRenderer.ts`,
> `src/services/math3d.ts` and the donor-era `src/App.vue`. That code no longer
> exists: the language moved to `crates/openscad-core` (checked against the
> official runtime by `tests/officialOpenScadRuntimeConformance.test.ts`), the
> renderer to `src/render/` with WGSL owned by `crates/raster-core/shaders`, and
> the shell to `src/App.svt` plus `src/components/` and `src/composables/`. The
> exporters cited in domain E (`stlExport.ts`, `objExport.ts`,
> `threemfExport.ts`) were replaced by kernel-backed `src/exchange/meshExport*.ts`.
> Only verified, still-open build/security/PWA/i18n items remain below. Live
> architecture debt is tracked in [recommendation.md](./recommendation.md).

A concrete catalog of things done badly or incorrectly, originally produced by a 5-domain deep audit. Each item cites a file and the actual problem.

Synced with: [README.md](../README.md) · [architecture.md](../architecture.md) · [RECOMMENDATIONS.md](./RECOMMENDATIONS.md) (fixes mapped to phases).

| Domain | File(s) | Count |
|--------|---------|------|
| E. Build / security / PWA / i18n | config, `index.html`, `public/` | 11 |
| **Total** | | **11** |

---

## E. Build / Security / PWA / i18n — 11 open

E10–E20 were resolved on 2026-11-14 (roadmap package P8) and stay struck through so the numbering is stable.

1. [build] package.json — no lint script/ESLint for TypeScript (Rust has `audit:clippy`).
2. [build] — no format script/Prettier.
3. [build] package.json — no packageManager field.
4. [build] tsconfig — no noUnusedLocals/Parameters.
5. [build] tsconfig — no noImplicitReturns/noFallthrough/noUncheckedIndexedAccess.
6. [build] tsconfig — no exactOptionalPropertyTypes/forceConsistentCasing.
7. [build] tsconfig — skipLibCheck masks dep type errors.
8. [build] tsconfig — no isolatedModules.
9. [artifacts] .gitignore — `src/**/*.js` too broad.
10. ~~[security] index.html — no CSP meta.~~ Resolved: The build injects a CSP meta tag (`scripts/pwa-build.mjs`); `public/_headers` sends it as a header.
11. ~~[security] index.html:16 — inline SW-registration script (CSP-hostile).~~ Resolved: Registration moved to `src/registerServiceWorker.ts`; `scripts/verify-dist.mjs` fails on any inline script in `dist/index.html`.
12. ~~[security] manifest — no CSP/permissions-policy/cache strategy.~~ Resolved: `public/_headers` documents CSP, `Permissions-Policy` and cache headers for the host.
13. ~~[sw] sw.js `handleAsset` — every same-origin 200 response is cached with no entry/size bound until the next version bump.~~ Resolved: Runtime cache bounded to 120 entries (oldest evicted) and 16 MiB per declared response.
14. ~~[sw] sw.js `install`/`activate` — unconditional skipWaiting + claim swaps SW mid-session (documented tradeoff, not gated behind an update prompt).~~ Resolved: No unconditional `skipWaiting`; a page may opt in with a `SKIP_WAITING` message.
15. ~~[sw] sw.js:6 — `CACHE_VERSION` hardcoded (`v2`); manual bump forgotten.~~ Resolved: `CACHE_VERSION` is a hash of the emitted files, stamped into `dist/sw.js` at build time.
16. ~~[sw] sw.js `fetch` — cross-origin requests (Google Fonts from index.html) are bypassed and never cached.~~ Resolved: Google Fonts stylesheet and font responses are cached.
17. ~~[pwa] manifest — single icon; no 192/512 PNG.~~ Resolved: 192 and 512 PNG icons in `public/icons/`.
18. ~~[pwa] manifest — SVG data-URI icon; rejected by iOS/some Android.~~ Resolved: Icons are PNG files (`scripts/generate-pwa-icons.mjs`).
19. ~~[pwa] manifest — maskable on non-padded SVG; art clipped.~~ Resolved: Separate full-bleed maskable icon with the art inside the safe zone.
20. ~~[pwa] index.html — no apple-touch-icon/iOS meta.~~ Resolved: `apple-touch-icon`, `theme-color` and Apple web-app meta tags added.
21. [i18n] src/i18n/appStrings.ts — ru/en dictionaries are `Record<string, string>`; no type or test keeps their keys in sync.
22. [i18n] — no RTL support anywhere in the app shell.

---

## How to use this list

- Fixes are grouped into phases in [RECOMMENDATIONS.md](./RECOMMENDATIONS.md). Mapping:
  - **E1–E8 (build/tsconfig)** → Recommendations **Phase 3.5**
  - **E9, E21–E22 (artifacts/i18n)** → no phase yet; take them individually. E10–E20 were closed by roadmap package P8.
