# Windows Companion status console

Internal private-preview implementation and validation, October 25, 2026.

The native status window follows the owner-supplied sidebar-console reference.
The design is implemented with Win32 controls, GDI drawing and Windows system
fonts. The reference HTML, bundled React files and JavaScript are not distributed.
No new runtime dependency or third-party artwork was introduced.

The sidebar opens Engines, Archive, Import and Reports folders and retains the
existing masked-token and signed-update workflows. The main view shows live
connection and engine counts, address, data folder, automatic imports and update
status. Copy address copies the HTTP connection URL, never the owner token.
Closing the status window leaves the background Companion running.

The window can be resized and maximized. Activity keeps its own read-only text
control, preserves selection and scroll position during updates, and uses the
gold scrollbar only when text overflows. Buttons support keyboard navigation.
Status values remain exposed through native accessibility controls.

## Validation

- `cargo test -p ironwood-server --bin ironwood-companion tray::`: five tests pass.
- Dashboard tests cover live values and child bounds at 940×760 and 1200×900.
- Existing gold-scrollbar rendering and overflow tests pass.
- Inspected the running native dashboard and verified Copy address.
- Packaged through `scripts/package-companion.ps1` into `output/companion/current`.
- Restarted with the existing configured data directory, preserving archive,
  registrations and token. No QNAP image change is required for this Windows UI.
