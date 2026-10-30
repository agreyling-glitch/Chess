# Ironwood Companion

Current version: **0.1.8**. Includes optional Windows sign-in startup, server-side
archive backups, and unified collection filtering. The public Windows release is **0.1.8**,
with release-key-verified updates.

The native Rust server foundation, UCI bridge, persistent game archive and archive
opening explorer and owner-local Syzygy probing are implemented. Tablebase probing
is opt-in through `tablebase_directories`; see [Phase 5 setup](docs/phase-5-tablebases.md).
The Windows x64 release is available from
[the Companion download page](https://ironwoodchess.com/downloads/companion/).
Linux, macOS and QNAP builds remain developer-supported.

## Windows quick start

Unzip the portable package and run `start-companion.cmd` or `ironwood-companion.exe`
for the Windows tray. The status window can be closed while Companion keeps running;
use the gold pawn tray icon for status, folder shortcuts, token controls, or Quit.
In the status window's **Settings** row, enable **Start Companion when I sign in**
to add a shortcut to your Windows Startup folder. Clear it to remove the shortcut.
This applies to your Windows account, needs no administrator permission, and starts
after sign-in. It preserves the current data directory and configuration path.
Keep the application folder in a stable location; if you move it, clear and enable
the option from its new location. Clear the option before deleting the portable app.
The token starts masked and is displayed only after Reveal; closing its window
removes the displayed value. Copy token uses the Windows clipboard, whose history
and synchronization tools may retain copied values.

Run `ironwood-server.exe` for terminal/headless operation, optionally with `--tray`.
First startup creates a private directory at
`%LOCALAPPDATA%\Ironwood\Companion`, with configuration and a random owner token.
Neither Node, Python, nor Rust is needed to run the packaged executable.

For development from the repository root:

```powershell
cargo build --locked -p ironwood-server
.\target\debug\ironwood-server.exe --data-dir tmp/companion-native
```

Open `http://localhost:8788/companion-check.html` in the normal Ironwood preview,
read the `owner-token` file privately, and enter it in the diagnostic. The page
never saves the credential or modifies games. Stop with Ctrl+C, by closing the
console, or authenticated POST `/api/v1/shutdown`.

## Shared browser connection

Open any Companion feature in Ironwood to connect. One connection dialog accepts
an address and token for engines, archive, opening explorer and local tablebases.
Each feature provides **Manage connection**. Switching to built-in Stockfish keeps
the shared connection available to the other features. **Disconnect all features**
clears it and resets an active Companion engine selection.

By default, the token stays in memory in that browser tab. Opt into **Remember
this connection on this device** when connecting to save the address and token in
local browser storage for this site. Reloads and new tabs restore those details;
services authenticate their requests normally. This stores the token on disk in
browser storage, so use it only on a trusted device. Different sites, browsers
and profiles have separate saved connections.

**Disconnect all features** ends this tab's connection but keeps saved details.
**Forget connection** removes the saved address and token and disconnects this
tab. Connecting with Remember unchecked removes previously saved details after
successful authentication. The developer diagnostic uses the same manager.

## Linux Docker

```sh
docker compose -f deploy/ironwood-server/compose.yaml up --build -d
docker compose -f deploy/ironwood-server/compose.yaml ps
```

The non-root container initializes a named volume at `/data`. It binds all
interfaces **inside** the container while Compose publishes port 8790 only on
host loopback. The root filesystem is read-only; only `/data` is writable.
Read the owner token privately through your Docker administration interface.
Do not paste it into logs or URLs. `docker compose down` retains the volume;
`down --volumes` deletes it and is not a normal upgrade step.

The Linux recipe is prepared but not runtime-validated in this environment.
Use versioned images/digests after release validation; none are published yet.

## Commands and operations

```text
ironwood-server init --data-dir <dedicated-directory>
ironwood-server --data-dir <dedicated-directory>
ironwood-server rotate-token --data-dir <dedicated-directory>
ironwood-server --config <existing-config.json>
ironwood-server healthcheck --address 127.0.0.1:8790
ironwood-server --help
```

`--docker` chooses the container binding only when first initializing. Existing
configuration is preserved. Rotation requires Companion to be stopped; it
atomically replaces the token, and reconnecting clients must enter the new one.
The legacy `scripts/companion-init.mjs` is superseded by native `init`.

See [operations and recovery](docs/phase-1-operations.md),
[protocol](docs/protocol-v1.md), [configuration schema](config.schema.json),
[licensing](docs/licensing.md), and [Phase 1 validation](docs/phase-1-validation.md).
See [UCI setup and Phase 2 validation](docs/phase-2-uci.md) to add engines through
the browser with owner-local CLI confirmation and no restart,
and select one through Game → Companion engines in Ironwood.
See [archive setup, transfers and recovery](docs/phase-3-archive.md) for
Storage → Companion archive, collections, PGN/JSON files, and SQLite snapshots.
See [archive opening explorer](docs/phase-4-explorer.md) for indexing, filters,
matching games and the Archive source in the existing Opening Explorer.
The [Phase 0 record](docs/phase-0-validation.md) retains the outstanding deployed
HTTPS-origin/browser matrix.

## Development checks and Windows package

```sh
cargo test --locked -p ironwood-server
cargo clippy --locked -p ironwood-server -p ironwood-protocol --all-targets -- -D warnings
cargo check --locked -p ironwood-protocol --target wasm32-unknown-unknown
cargo build --locked -p ironwood-server
node --test tests/companion-server.test.mjs tests/companion-lifecycle.test.mjs
node scripts/companion-licenses.mjs
```

On Windows, run `scripts/package-companion.ps1` with PowerShell to create an
unsigned development ZIP, checksum, and matching source including vendored
dependencies. The permanent launcher is `output/companion/current/start-companion.cmd`.
Future builds reuse that location and `output/companion/ironwood-companion-windows-x64.zip`.
Stop Companion before packaging an update. Packaging builds in a temporary folder
and replaces `current` only after the new package is complete. Owner data remains
in the configured data directory, outside the build. `-OutputDirectory` can choose
another destination inside the repository's `output` folder.

The native tests use disposable directories and local ports. Set
`COMPANION_BINARY` to test an extracted executable. Linux and clean-machine
acceptance remain separate from Windows development-machine success.

Real Syzygy comparison instructions and recorded Windows development evidence:
[Phase 6 tablebase validation](docs/phase-6-tablebase-validation.md).

Portable-package results and Linux Docker/clean-Windows runners:
[Phase 6 platform validation](docs/phase-6-platform-validation.md).


## Server-side archive backups (0.1.6+)

Set **Backup folder** in Companion's Windows status window and choose **Save folder**.
You can also configure the selected server using **Settings → Companion connection → Backup folder…** or **Game History → Backup and Restore… → Configure backup folder…**.
The absolute path belongs to the Companion machine, not the browser. A NAS share or mounted volume must be readable and writable by the account running Companion. The default is `backups` beside the archive database. JSON configuration uses `backup_directory` (relative paths resolve beside config.json).

Game History's Backup and Restore window creates SQLite snapshots directly in that folder. Restores select an existing snapshot from the same folder; to restore on another machine, copy the snapshot into that Companion's configured backup folder first. Archives of millions of games never pass through browser memory. Snapshots include games, original data, analyses, collections and indexes, but not owner tokens, engine executables, or machine configuration. Browser backups retain their separate JSON format.

One backup or restore runs at a time. The archive is reserved while copying or validating, and folder imports wait; engines keep running. Jobs continue if the browser window closes. The window polls progress and reports completion or failure. Snapshots are integrity-checked before publication; incomplete files are not offered for restore. Restore validates the snapshot, requires the current archive to have no games or collections, and uses SQLite's transactional backup API. Backup files are never silently replaced. There is no automatic retention/deletion policy.

API (owner token, exact Origin and protocol header required):
- `GET /api/v1/archive/backups/settings`: configured folder.
- `POST /api/v1/archive/backups/settings` with `{"folder":"absolute server path"}`: persist folder.
- `GET /api/v1/archive/backups`: available `.sqlite3` files and current job/progress.
- `POST /api/v1/archive/backups` with `{"action":"backup"}` or `{"action":"restore","file":"listed backup filename"}`: start a background job.


## Collections in Saved Games (0.1.8+)

Saved Games manages collections for Browser and each selected Companion: create, rename, delete (keeping games), filter, and add/remove games. Games can belong to multiple collections. Browser backups preserve the local collection registry and membership; Companion SQLite snapshots already include collections and memberships. Copy/move dialogs choose a destination collection, and moves remove source games only after destination storage and membership have succeeded.

Companion 0.1.8 adds the indexed `collection` filter to archive/library queries and invalidates cached counts after membership changes. Older Companions must be updated for filtering in the unified library. Collection CRUD continues to use revision-checked archive/collections endpoints. The old archive window no longer exposes collection management.
