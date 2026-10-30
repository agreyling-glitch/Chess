# Installing Ironwood Companion on QNAP Container Station

Developer-supported QNAP instructions, updated October 30, 2026.

Run Ironwood Companion on the QNAP NAS and connect to it from Ironwood on your
Windows computer. This setup uses the NAS address `192.168.0.2`, Container Station 3,
and the image `ironwood-companion:qnap-preview`. Substitute your NAS address if it
differs. The build was exercised on a QNAP TS-677 with a Ryzen processor, targeting
Linux x64. Other NAS architectures need their own build.

Companion remains behind the preview checkbox in Ironwood's
**Help → About → Version diagnostics**. These instructions are for private testing.

## What you need

- Container Station installed and running on the NAS.
- SSH enabled on the NAS for the build procedure.
- A NAS account permitted to run Docker. This setup used `admin`.
- Internet access from the NAS to download build dependencies and base images.
- The prepared source kit or a complete Ironwood source checkout, including
  `vendor`, `assets/openings`, and `tests/fixtures`.

The Windows and QNAP Companions are separate installations. They have separate
archives, tokens and engine registrations. Creating the NAS container does not
move games from the Windows archive.

## Build the image on the NAS

Skip this section if `ironwood-companion` with tag `qnap-preview` already appears
under **Container Station → Images**.

### Use the prepared Windows runner

The runner expects this source kit in the local repository:

```text
output/companion-qnap/ironwood-companion-qnap-build.tar.gz
```

Open PowerShell on Windows and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\Dad\git\Chess\scripts\build-companion-qnap.ps1 -NasHost 192.168.0.2 -Username admin
```

Enter the NAS password only at the SSH/SCP prompts. Password characters do not
appear while typing or pasting; press Enter after entering the password. The
runner does not save the password. It may prompt more than once during upload,
building and download. The example assumes the NAS SSH host key is already trusted;
verify the NAS identity if establishing SSH access for the first time.

The runner creates a dedicated directory under `/share/Public`, uploads the source,
and builds the Linux x64 image on the NAS. It tests startup and token persistence
through a restart using disposable storage. It then downloads these files to
`C:\Users\Dad\git\Chess\output\companion-qnap`:

- `ironwood-companion-qnap-linux-amd64.tar.gz`
- `ironwood-companion-qnap-linux-amd64.tar.gz.sha256`
- `image-info.json`
- `build.log`

The runner verifies the downloaded image archive's checksum. The NAS build directory
is retained and its path is printed. These checks establish build, startup and
restart behavior; they are not a full Linux functionality certification.

### Build from a complete source checkout

Alternatively, in an SSH session on the NAS, change to the source root and run:

```sh
sh deploy/ironwood-server/qnap-build.sh
```

The script uses two compilation jobs. During compilation, detailed output goes to
`build.log`, so the terminal may remain quiet for several minutes. Successful
completion prints the exported image filename.

## Find the correct image

Open **Container Station → Images** and find:

| Field | Value |
| --- | --- |
| Name | `ironwood-companion` |
| Version or tag | `qnap-preview` |

A build performed on this NAS already adds the image to Docker. No import is
needed. **Unused** means no container currently uses the image; it is not an error.

Older images named `ironwood-companion-validation`,
`ironwood-companion-validation-builder`, and `ironwood-companion-validation-tools`
are test tools. Select `ironwood-companion:qnap-preview` for this installation.

If transferring the image to another compatible NAS, use **Images → Import**.
The `.tar.gz` is a Docker image archive. **Containers → Import** expects a different
kind of export and can reject it with **Invalid File Format**.

## Prepare folders visible in File Station

The setup uses the existing **Container** shared folder and adds two folders:

| File Station location | NAS path | Container path |
| --- | --- | --- |
| Container → Ironwood → Import | `/share/Container/Ironwood/Import` | `/data/Import` |
| Container → Ironwood → Reports | `/share/Container/Ironwood/Reports` | `/data/Reports` |

Create and prepare these folders before applying the YAML. The archive,
configuration and owner token remain in the private named volume at `/data`.
Only Import and Reports are exposed through the shared folder.

### Existing Companion installation

Run this in your own Windows PowerShell window:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\Dad\git\Chess\scripts\configure-qnap-shared-folders.ps1 -NasHost 192.168.0.2 -Username admin
```

The script asks for the NAS password through SSH. It locates the Companion
container in the `ironwood-companion` application, stops it, and copies all
existing Import and Reports contents to the two new folders. It keeps the
original files in the old volume and refuses to overwrite nonempty destination
folders. It changes ownership to UID/GID 10001, the container's service account,
throughout those two folder trees, including copied queue subfolders and files.
It grants the owner read/write access and directory traversal, then tests access
using the service account. Symbolic links are rejected before ownership changes.

Companion stays stopped after the copy. Recreate the existing application using
the YAML below, preserving the same named volume at `/data`. Confirm the container
is healthy before dropping in more files. Do not restart the old configuration
and continue importing between the copy and the mount change, because the copies
would diverge. If setup fails, keep both copies and review the reported error.

Adding a bind mount hides the old folder contents from the container; it does not
move them. That is why copying existing contents before changing the mounts is
necessary. The original volume is retained for recovery; its old Import and
Reports folders are no longer the active folders once the new mounts are applied.

### New Companion installation

Before creating the application, connect to the NAS with SSH and prepare empty
folders:

```sh
mkdir -p /share/Container/Ironwood/Import /share/Container/Ironwood/Reports
chown 10001:10001 /share/Container/Ironwood/Import /share/Container/Ironwood/Reports
chmod 750 /share/Container/Ironwood/Import /share/Container/Ironwood/Reports
```

These commands are for new, dedicated folders. Do not apply ownership changes
recursively to the Container share or to Container Station's storage directories.
Use the NAS `admin` account in File Station for this setup. Access by other NAS
users requires separately granting the appropriate shared-folder/file permissions;
some generated report files are private to the service account. Keep service UID
10001 able to read, write and enter both mapped folders. A permission error should
be resolved on these folders, rather than by running Companion privileged.

## Create or update the Companion application

1. Open **Applications → Create** in Container Station.
2. Set the application name to **`ironwood-companion`**, using lowercase letters.
3. Paste the following YAML or upload `deploy/ironwood-server/compose.qnap.yaml`.
   For an existing installation, use its Recreate action to update the YAML instead
   of creating a second application.
4. Validate the YAML.
5. Leave CPU and memory limits unset for the unlimited-resource setup used here.
6. Click **Create** and wait for the application to show **Running**.

```yaml
services:
  companion:
    image: ironwood-companion:qnap-preview
    ports:
      - "8790:8790"
    volumes:
      - ironwood-companion-qnap-data:/data
      - /share/Container/Ironwood/Import:/data/Import
      - /share/Container/Ironwood/Reports:/data/Reports
    read_only: true
    tmpfs:
      - /tmp:rw,nosuid,nodev,size=256m,mode=1777
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    stop_grace_period: 30s
    restart: unless-stopped

volumes:
  ironwood-companion-qnap-data:
```

The `/data` volume holds the archive, configuration, connection token and engine
files. The two additional mounts make the shared folders the active Import and
Reports locations. Container Station may prefix the named volume name
with the application name. Keep this volume when recreating or updating the
container. The container's read-only setting protects its program files; `/data`
remains writable.

In File Station 6, open **Container → Ironwood** to upload to **Import** and read
**Reports**. Use copies of your source files: successful automatic imports consume
their input files after saving the games and report. Keep your originals elsewhere.
Do not change or remove a file while Companion is importing it. The app continues
to display import progress using the same `/data/Import` location inside the
container, even though its NAS storage is now shared.

Container Station 3 manages resource limits through **Advanced Settings →
Resources**. Do not add `mem_limit`, `pids_limit` or a resource-limits section to
this QNAP YAML. If you later want a memory or CPU limit, use the QNAP controls.

## Configure access from the Windows computer

The port mapping in the YAML above makes Companion reachable at:

```text
http://192.168.0.2:8790
```

| Port mapping | Who can connect |
| --- | --- |
| `"8790:8790"` | Clients able to reach a NAS network interface, including its LAN address |
| `"192.168.0.2:8790:8790"` | Clients able to reach that specific NAS address |
| `"127.0.0.1:8790:8790"` | Programs running on the NAS itself, or clients using an SSH tunnel |

The initial sample used `127.0.0.1`. That prevented the Windows computer from
connecting directly to `192.168.0.2:8790`. For ordinary LAN access, use
`"8790:8790"`, as in the other NAS containers in this setup. In Docker inspection,
this normally appears as host address `0.0.0.0` for the published IPv4 port.

To change an existing application, open its **Recreate** action from the application
menu, update the YAML, and apply the update. Preserve the existing volume mapping.
Changing the published port requires recreating the container; a restart alone
does not change it. No image rebuild is needed.

Use this HTTP endpoint on your trusted LAN. Keep router port forwarding disabled
for port 8790. A NAS firewall must allow the Windows computer to reach that port.

### Optional SSH tunnel

If you intentionally keep the NAS-only mapping, run this on Windows and leave the
PowerShell window open:

```powershell
ssh -N -L 127.0.0.1:8791:127.0.0.1:8790 admin@192.168.0.2
```

Then use `http://127.0.0.1:8791` as the QNAP connection address in Ironwood.
Port 8791 avoids the Windows Companion already using port 8790. The tunnel is
unnecessary when connecting directly through the LAN mapping.

## Retrieve the QNAP connection token

1. Open **Containers** and select the container belonging to the
   `ironwood-companion` application.
2. Use **Execute** to open `/bin/sh` inside that container.
3. Run:

   ```sh
   cat /data/owner-token
   ```

4. Copy the value into Ironwood's connection-token field. Keep it private.

**Owner credential** and **connection token** mean this same value. It is not the
NAS admin password, and it is not the Windows Companion token.

Companion generates a token when initializing a new data directory. The same token
survives restarts, port changes and container replacement when `/data` is retained.
Explicit token rotation or a fresh data directory changes the token.

## Connect Ironwood to the QNAP Companion

In the Windows Ironwood app, enable the Companion preview checkbox if necessary,
then open **Game → Companion connection** and add a connection:

| Setting | Value |
| --- | --- |
| Companion name | `QNAP` or `NAS4` |
| Companion address | `http://192.168.0.2:8790` |
| Connection token | Token retrieved from this container |
| Default archive | Enable to use the NAS archive for saved games, research and tablebases |
| Use as an engine provider | Enable only if you intend to run engines on the NAS |
| Remember this connection | Optional; saves the connection on this browser |

Click **Connect / Save**. For the tunnel setup, substitute
`http://127.0.0.1:8791` for the address. Windows Companion can remain registered as
an engine provider while QNAP supplies the default archive.

The container starts with an independent archive. Existing Windows games must be
transferred or imported separately if you want them on the NAS. The current
Auto-install engine catalog has no Linux packages, so NAS engines require manual
registration of compatible Linux executables; Windows `.exe` engines do not run
in this image.

## Troubleshooting

| Symptom | Action |
| --- | --- |
| Invalid File Format during import | Use **Images → Import**, or use the image already built on the NAS. |
| Incompatible Docker Compose YAML File mentioning Resources | Remove YAML resource limits; configure them in **Advanced Settings** if wanted. |
| YAML passes but application name stays red | Use lowercase `ironwood-companion`. |
| Failed to fetch in Ironwood | Check the full `http://` address, LAN port mapping, NAS firewall, and any browser local-network permission prompt. |
| Owner credential required when opening the address in a browser tab | The service is reachable but the tab supplied no token. Connect through Ironwood with the QNAP token. |
| Unauthorized or invalid credential in Ironwood | Retrieve the token from the current Ironwood container; do not use the NAS password or Windows token. |
| Origin rejected | Use an allowed Ironwood origin. This build initially permits `https://ironwoodchess.com`, `http://localhost:8788`, and `http://127.0.0.1:8788`. Other app origins require an explicit configuration change and restart. |
| Archive or token appears to have reset | Check that the recreated container uses the original `/data` volume. Preserve the old volume while investigating. |
| Mapped folder is missing or permission is denied | Create both host folders before recreation. Check that service UID 10001 can read/write files and enter subfolders, including .processing and Failed. For an existing mapped installation, use [Repair shared-folder permissions](#repair-shared-folder-permissions). |
| Import or Reports appears empty after adding mounts | The new shared folders hide the old volume folders. Stop Companion and preserve both copies before checking whether the migration step was completed. |
| Container is Running but the app cannot connect | Check container health and logs; Running alone does not establish authenticated browser connectivity. |

## Update and preserve data

Build the new image, recreate the existing application using that image, and keep
the same volume at `/data`. Back up the archive before updating an installation
that contains games. Do not delete the data volume when removing an old container.
The exported image archive is a backup of the program image, not an archive backup.

## Reference files

- [QNAP Compose example](../../deploy/ironwood-server/compose.qnap.yaml)
- [Windows build runner](../../scripts/build-companion-qnap.ps1)
- [NAS build and smoke test](../../deploy/ironwood-server/qnap-build.sh)
- [Shared folder setup runner](../../scripts/configure-qnap-shared-folders.ps1)
- [NAS shared folder migration](../../deploy/ironwood-server/qnap-share-folders.sh)
- [Windows permission repair runner](../../scripts/repair-qnap-folder-permissions.ps1)
- [NAS permission repair script](../../deploy/ironwood-server/qnap-repair-folder-permissions.sh)
- [Container image definition](../../deploy/ironwood-server/Dockerfile)
- [QNAP Container Station 3 guide](https://www.qnap.com/en-us/how-to/tutorial/article/how-to-use-container-station-3)

## Repair shared-folder permissions

If automatic imports show `Queue paused: Permission denied (os error 13)` after
mapping the shared folders, copied `Processing` or `Failed` subfolders may retain
an old owner. Use this repair for folders already mapped into the running
container; do not rerun the migration script against nonempty shared folders.
From the repository in PowerShell, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\repair-qnap-folder-permissions.ps1 -NasHost 192.168.0.2 -Username admin
```

Enter the NAS password at the SSH prompt. This briefly stops only the Ironwood
container, changes ownership to service UID/GID 10001 throughout the mapped
Import and Reports folders, checks directory access, and restarts the container
if it was running. It does not delete files or modify the archive or token.
The migration setup now applies ownership to copied subfolders too.

Refresh the app after the container restarts. The queue should leave the permission
error and return to waiting for files or continue importing. If the error remains,
check QNAP shared-folder ACLs for service UID 10001; Unix ownership alone cannot
override an explicit ACL denial. Do not change ownership on the whole Container
share or Docker's private volume directories.

This folder repair requires no new image, YAML change, or token rotation. Future
Companion program updates still require rebuilding the image and recreating the
application with the same data volume and shared-folder mappings.

## Writable temporary storage

The read-only container root requires a separate writable `/tmp`. The Compose
example mounts a temporary RAM filesystem there, limited to 256 MiB. This is a
temporary-files limit, not a limit on the archive or the container's total memory.
Temporary contents disappear when the container is replaced; archive data stays
in the existing `/data` volume.

If diagnostics report `Not writable: /tmp`, recreate the existing application
with the updated Compose YAML, preserving all existing volume mappings. Restart
alone does not add the mount; no image rebuild or new connection token is needed.
Then run `scripts/diagnose-qnap-import.ps1` again and confirm `/tmp` is writable.
An archive-save failure still needs investigation if it persists after this change;
the generic error alone does not identify its SQLite cause.

For a failed ZIP import, keep the original ZIP and its report. After correcting
the configuration, copy the ZIP back into the top-level Import folder to retry.
The queue's internal working folder is named `.processing` (with a leading dot).


## Upgrade an existing installation to 0.1.6

Use the prepared **upgrade** runner rather than creating another application:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\Dad\git\Chess\scripts\upgrade-companion-qnap.ps1 -NasHost 192.168.0.2 -Username admin
```

It uses `output/companion-qnap/ironwood-companion-qnap-0.1.6-build.tar.gz` and verifies the source checksum after upload. Enter the NAS password only in the SCP and SSH prompts. Progress and the final status are saved locally in `output/companion-qnap/upgrade-session.log` and `upgrade-status.json`.

The runner identifies the existing `ironwood-companion` application and records its actual configuration and mounts. It builds a versioned `ironwood-companion:qnap-0.1.6` image while the existing service stays running, tests Linux backup code, and checks health and token persistence using disposable storage. It then checks available recovery space, stops only Companion, and writes a private compressed recovery copy of all `/data` content, including nested Import/Reports mounts. This recovery copy contains the owner token: keep its private NAS directory protected and do not share it.

Only the application's image field is changed. All existing volumes, bind mounts, settings and ports are retained. After recreation, the runner checks health, exact mount identity, and unchanged config/token checksums. The previous image and recovery files remain on the NAS. It does not remove volumes or automatically prune old images. If the application uses an unsupported compose layout, it stops rather than guessing. If verification fails after recreation, leave the volumes and recovery copy intact and review the reported error before starting or restoring anything.

0.1.6 adds server-side SQLite backups for large archives. Reconnect the QNAP in Ironwood and open **Game History → Backup and Restore…**. Its default backup folder is inside `/data`, so it uses the existing persistent data volume. Folder settings must name a path **inside the container**. To use a different NAS shared folder, first add a writable bind mount for that folder and grant access to UID/GID 10001; preserve every existing mount. Do not use an unmounted host `/share/...` path as the container's backup setting.


If a source kit has already been uploaded, the runner accepts `-ResumeRemoteDirectory /share/Public/ironwood-companion-upgrade-<recorded identifier>`. It uploads a small corrected script bundle and uses a short SSH command, avoiding long encoded command lines. Enter the SCP and SSH passwords locally. The NAS job is detached from the SSH transport, keeps a private `upgrade-run.log`, and writes an `upgrade-result` exit code. A running-job lock prevents a duplicate upgrade; reconnecting attaches to its existing log. Do not manually start a second recreation while a job is active. Local streamed output is in `output/companion-qnap/upgrade-remote.log`.
