# Private-preview registered engine management

Removal: the manager reads saved configuration without launching the engine.
Save still validates launch changes. An explicit removal confirmation keeps all
profiles, ratings and games; unavailable engines must be restored before their
profiles can start. Manual registrations always keep files. Companion publisher
packages offer optional deletion, checked against their catalog receipt, hashes,
canonical installation root, other registrations, extra files and links. All engine
slots and registration changes are reserved during removal. Uninstall moves the
verified package aside, commits configuration removal, and restores it on commit
failure. Partial cleanup is reported. Download caches are retained for reinstall.
Authenticated POST `/api/v1/engine-management/<id>/remove` requires the current
configuration token and a boolean `delete_files`. The engine ID remains unchanged
until removed; no browser training/history storage is edited.

UI revision: Manage engines now focuses on the friendly name and launch settings.
It opens the selected Engine Settings identity directly, with no second provider
or engine selector. The heading shows the selected engine name. Built-in engines
do not enable the Companion manager, and unknown identities never fall back to
another registered engine.
The executable remains read-only, and launch arguments are always visible.
Playing/analysis UCI controls are provided by Engine Settings → Engine options;
the manager no longer duplicates those controls. Existing saved Companion defaults
are preserved unchanged when saving a name or launch-argument edit.

Engine Settings now has Manage engines beside Add engine. The manager selects an
enabled Companion provider and registered engine, initially using the current
selection when available. It edits the friendly name, launch arguments and saved
UCI defaults. The registered executable is shown read-only. Each editable option
has an explicit Override switch; inherited defaults are not silently persisted.
Spin/check/combo/string controls follow the engine's advertised types and effective
Companion resource ranges. One-time UCI buttons are not saved as startup defaults.
Network/tablebase paths refer to the Companion host.

Existing Play/Realtime/Full-game Engine options remain per-context controls. Saved
Companion defaults affect new processes; already-running sessions retain their
settings. Reselect/reconnect the engine after saving to obtain a new session.
Explicit play/analysis overrides continue to take precedence over saved defaults.

Owner-authenticated GET/POST `/api/v1/engine-management/<registeredId>` reads and
updates settings. GET inspects UCI options in a separate bounded engine process;
if the current configuration cannot start, saved values/arguments remain editable
so the owner can repair them. POST validates a candidate process before persisting.
Unknown options, wrong types, out-of-range values, command injection and startup
button actions are rejected. Incoming data cannot change the executable or ID.

The existing registration reservation serializes validation/configuration changes.
A digest of the complete configuration provides optimistic concurrency protection;
both the on-disk configuration and live registry are checked before replacement.
Only the selected engine's label/arguments/defaults are replaced. All other
engines and configuration fields are retained. The write uses a private unique
temporary file and atomic replacement, updating the registry only after success.
Direct edits to the same engine outside Companion require a restart before using
the manager. Read-only inspection does not persist changes or alter active sessions.

The shared Companion flag gates loading/actions and closes the management dialog
when disabled. Connection changes and closed dialogs invalidate pending replies.
The window reuses Ironwood's resizable, movable gold/grain engine-dialog styling.

Validation: real isolated HTTP tests edit spin, combo, check and string defaults,
friendly names and arguments; restart confirms persistence. Invalid values,
startup actions, injected text, executable spoofing and stale configuration
tokens are rejected without changing the file. Other engine entries survive
unchanged. UI tests cover typed controls, explicit overrides, inherited defaults,
failure recovery, feature gating and successful/unsaved close results.
