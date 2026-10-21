# Companion foundation: operations and recovery

This records the Phase 1 foundation. The current server also includes the
[UCI bridge](phase-2-uci.md) and [persistent archive](phase-3-archive.md);
use the archive snapshot instructions when backing up game data.

## Provisioning and storage

`ironwood-server` starts with native defaults. Windows uses
`%LOCALAPPDATA%\Ironwood\Companion`; Linux defaults to `./companion-data`, while
the Docker command explicitly uses `/data`. Use `--data-dir` to choose a dedicated
directory. First startup and `init` share the same provisioning path. `init`
does not listen on a port. Existing valid configuration/credentials are preserved.

First run creates `engines/` and `archive/` inside that data directory. Keep each
installed engine in its own subfolder (for example `engines/lc0/`), including its
supporting files. Engine processes start with that subfolder as their working
directory. The default database is `archive/archive.sqlite3`.

Existing default `archive.sqlite3` databases are checkpointed and moved on startup,
under the data-directory lock, with an atomic config update and recovery journal.
Games and credentials are preserved. Close external SQLite clients before the
move. A conflicting destination is never overwritten, and explicitly configured
custom database paths and disabled archives remain unchanged.

Initialization refuses unrelated nonempty uninitialized directories (an existing
engines folder or empty archive folder is allowed) and never replaces an
existing token. If first initialization is interrupted after creating a token
but before writing config, preserve the token and repair configuration manually
from `config.schema.json`, or move the dedicated directory aside and initialize
a fresh directory. Do not discard any directory containing user data.

Linux creates directories with mode 0700 and credentials/config with mode 0600.
Windows removes inherited directory permissions and grants full inherited access
to the current owner's SID using OS `whoami`/`icacls` tools without a shell or
visible helper windows. This is for a dedicated user-owned directory, not shared
folders. Existing explicit directory grants are not removed; administrators
adopting an existing directory must check those grants. Disk administrators and
a compromised operating system remain outside this protection boundary.

The OS file lock on `companion.lock` prevents a second instance or credential
rotation while the same data directory is open. Keep the lock file in place;
the OS releases the lock after process exit or crash. Do not run multiple servers
against copies of a live data directory or on unsupported network storage.

On Windows, `start-companion.cmd` passes `--start-if-needed`. If the same data
directory is already locked, the executable prints “Companion is already running.”
and returns launcher status 20; the script pauses to keep that message visible,
then returns success. Other startup errors retain their error status and details.

`--config <path>` preserves Phase 0 compatibility and locks the configuration's
parent directory. This mode also prepares the engines/archive subfolders and
migrates the legacy default database, without changing the parent ACL. Use
native initialization for new installations. A user-selected token path is
trusted local configuration; it is never accepted over HTTP.

## Credential rotation

1. Stop Companion and confirm the process exits.
2. Run `ironwood-server rotate-token --data-dir <directory>`.
3. Restart and pair browsers using the new file contents.

Rotation writes and flushes a private temporary credential, then atomically
replaces the existing credential. The owner credential is never printed. Rotation
manages a token filename directly inside the data directory; external/nested
token paths must be migrated deliberately. Old tickets disappear when the old
process exits. There is no remote rotation endpoint or automatic credential reset.

## Health, readiness and shutdown

- GET `/healthz`: public minimal liveness (`{"status":"ok"}`).
- GET `/readyz`: public foundation readiness, or 503 while stopping. These expose
  no engines, credentials, paths, archive data, or configuration.
- `ironwood-server healthcheck [--address IP:port]`: bounded native readiness probe;
  used by the image HEALTHCHECK without adding curl or Node.
- POST `/api/v1/shutdown`: requires the same Origin, owner credential, and protocol
  header as other API requests; returns 202 and starts graceful shutdown.
- Ctrl+C and Windows console-close events trigger the same path. Unix SIGTERM
  supports `docker stop`. Windows service/tray integration is not implemented.

Shutdown marks readiness false, refuses new API work, stops accepting connections,
signals upgraded sockets (including clients waiting to authenticate), and waits
for HTTP work/socket permits to drain within the configured deadline. No engine
or archive jobs exist yet. Future services must subscribe to this cancellation
path; it is not a claim that engine-process cleanup is already implemented.
The process/runtime terminates remaining connection tasks at the deadline.

## Resource limits

All limits live under optional `limits` in `config.json`; omitted fields retain
defaults. Invalid/out-of-range values fail startup. Restart to apply changes.

| Setting | Default | Meaning |
| --- | --- | --- |
| `max_connections` | 16 | Concurrent upgraded WebSockets, authenticated or not |
| `max_pending_tickets` | 64 | Unconsumed tickets; expired entries are reclaimed |
| `requests_per_minute` | 240 | Fixed-window HTTP budget per peer IP, including preflights |
| `max_request_bytes` | 4096 | HTTP request body / WebSocket message and frame limit |
| `request_timeout_seconds` | 10 | Body reading and HTTP handler deadline |
| `authentication_timeout_seconds` | 5 | First WebSocket authentication frame deadline |
| `idle_timeout_seconds` | 30 | Maximum time waiting for the next socket frame |
| `shutdown_timeout_seconds` | 5 | Shutdown drain deadline |

There are at most 1024 tracked rate-limit peers; old entries expire after 60
seconds. Forwarded headers are not trusted. Clients sharing a proxy/IP share
the budget. Health probes bypass the API budget so load cannot hide liveness.
The current HTTP endpoints accept no request bodies; oversized requests return
413 and unexpected bodies return 400. Allow enough bytes for a ticket-bearing
authentication message; very small limits intentionally make WebSockets unusable.

These are application limits for a loopback-first foundation. Raw TCP admission,
TLS, proxy authentication, and public-internet rate control are not provided by
this process. Remote deployments remain outside initial support. Docker adds
memory and process limits, drops capabilities, uses no-new-privileges, and runs
as UID/GID 10001 with a read-only root filesystem.

## Logs, updates and backup

Logs are newline-delimited JSON: startup/configuration, listening address,
request status/duration, shutdown and stop. No request body, URL/query, Origin,
Authorization header, token or socket ticket is logged. Configuration parse
errors use fixed explanations rather than echoing submitted JSON. No disk log
file grows inside the data volume; rotate container/host logs using the host's
normal facilities.

Back up the dedicated data directory while Companion is stopped. It currently
contains only configuration, credentials, and a reusable lock file; archive
database backup/restore belongs to Phase 3. Treat backups as secrets. Preserve
data separately from executables and Docker image layers.

Compose uses `restart: unless-stopped`; an API shutdown may be restarted by Docker.
Use `docker compose stop` for maintenance or rotation, rather than the API stop.

For native upgrades, stop, back up data, replace the executable, and restart with
the same directory. For Docker, stop and recreate the service using the same named
volume. Never use `down --volumes` for an upgrade. Rollback may use the prior binary
while configuration is compatible; future schema migrations need their own
recovery gate. This version performs no archive migrations and has no auto-updater.

Windows packaging includes an unsigned executable, launcher, checksum, docs,
license and exact source with vendored dependency license/source files. The
included source workspace preserves the release profile but excludes the browser
application's unrelated dependency graph. Linux image publication still requires
runtime validation, base digest pinning, and OS package licensing/source review.

## Windows desktop shell (private preview)

The portable package includes `ironwood-companion.exe` (Windows GUI subsystem)
and `ironwood-server.exe` (console subsystem). The launcher starts the GUI entry
point without a console; both use the same server, configuration and directory
lock. An existing instance is reported rather than starting a second server.
The CLI remains headless by default; `--tray` is opt-in and `--no-tray` is explicit.
Docker/non-Windows operation does not load desktop APIs.

The original gold pawn icon reports Ready, Engine active, and Stopping. The status
window shows the listening address, version, connections, engine sessions and
registered engine count. Its Close button keeps the server running; Quit requests
the ordinary graceful shutdown. Explorer restarts restore the notification icon.

Credential actions run only in the local process. No HTTP route, URL, process
argument, tooltip, notification, or log contains the owner token. The token edit
control contains a masked placeholder until Reveal is clicked; Hide and closing
the window remove the displayed credential. Copy uses Unicode clipboard data;
clipboard history/sync may retain it. No password prompt or credential rotation
is added. The native bindings are windows-sys 0.61.2 (MIT OR Apache-2.0), already
present in the dependency inventory, with sources/notices included in packaging.
