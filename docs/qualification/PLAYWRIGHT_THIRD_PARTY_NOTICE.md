# Playwright qualification dependency notice

- Packages: `playwright`, `playwright-core`, optional `fsevents`
- Version: `1.62.1`
- Source: <https://github.com/microsoft/playwright/tree/v1.62.1>
- License: Apache License 2.0
- License text: <https://github.com/microsoft/playwright/blob/v1.62.1/LICENSE>
- npm integrity for `playwright@1.62.1`:
  `sha512-0M+L3LAD8/nm554LOla9Ayx0j0tmFZ0FBcoQ7F1VuVHpM/XpiC8RcDzBQB8W5+hA8L22THxELzeF+2WcUzvcLg==`
- npm integrity for `playwright-core@1.62.1`:
  `sha512-wPYSwEBJY9GHraISXqyqtx0na0LpO3XEX7jNDhntbex7tzUS7kLnZsOlFruFJB4Hi/rhDMjXGqHewDZ68nYZVw==`

The qualification lock also contains the macOS-only optional package
`fsevents@2.3.2`, licensed under MIT, with npm integrity
`sha512-xiqMQR4xAeHTuB9uWm+fFRcIOgKBMiOBP+eXiyT7jsgVCq1bkVygt00oASowB7EdtpOHaaPgKt812P9ab+DDKA==`.
Its source and license are recorded at
<https://github.com/fsevents/fsevents/tree/v2.3.2>.

The private qualification manifest requests npm `10.9.8` as its provisioning
CLI. npm is licensed under Artistic License 2.0; its source is
<https://github.com/npm/cli/tree/v10.9.8>. The exact executable/toolchain
identity is deliberately not claimed here and remains a blocking runtime
binding in qualification row `u02`.

Playwright is used only by the G1 qualification harness. Its managed browser
binaries are separate third-party artifacts: they are not part of either npm
lock or the product bundle. A qualification environment must record each
binary's identity and license metadata before treating a browser run as
evidence; this remains blocking row `u03`.

This notice does not amend the immutable Manifold production dependency
evidence in the repository-level `THIRD_PARTY_NOTICES.md`.
