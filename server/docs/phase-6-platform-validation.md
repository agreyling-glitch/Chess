# Phase 6: platform validation

Status on 2026-10-17: portable-package validation passed on the Windows x64
**development machine**. Linux Docker and a genuinely clean Windows VM have not
been run. The available PC is Windows 11 Home without Docker, WSL or Windows
Sandbox. The owner reports a QNAP TS-677 with AMD Ryzen 5 and Container Station. Its SSH
endpoint is reachable and its saved host key is accepted, but existing key/agent
authentication was rejected for the specified account. NAS Docker version,
resources and actual architecture have not yet been read. NAS execution awaits
an existing usable SSH identity or an owner-authenticated run. No NAS containers,
shares, authentication settings or existing data have been changed.

## Windows evidence

The unsigned development package is
`output/companion-phase6b/ironwood-companion-0.1.0-windows-x64.zip`, with its
SHA-256 sidecar and matching vendored source. Its package hash is
`4a0590a6c094fb14891dd5562feb268879526ac80ad800f3c69d4d6cf74a958b`.
The executable hash is
`fa4659edd113362ea3f11a59a9a4d54790810273da62363bd588a796695ca8d5`.
These identify this development snapshot, not a published/signed release.

A dependency scan found that the earlier default MSVC build imported
`VCRUNTIME140.dll`. Windows packaging now builds the explicit x64 MSVC target
with static CRT linkage, and preserves that setting in the source bundle's
`.cargo/config.toml`. The packager restores its environment flag afterward.
The resulting normal PE imports are only Windows system DLLs; no redistributable
runtime DLL is required by that import list. Static linkage follows
[the Rust reference](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes).
This does not certify every Windows version or substitute for a clean OS test.

Recorded evidence:

- [Portable smoke report](../validation/windows-x64-portable-package.json): inbox
  Windows PowerShell 5.1 extracted and checked the ZIP, started the executable
  with a restricted PATH, checked private ACLs, authentication/origins, all four
  service flags, archive/explorer/local probing, UCI via an original inbox
  PowerShell fixture, restart persistence, rotation and graceful shutdown.
- [PE import report](../validation/windows-x64-portable-imports.json): normal
  imports reported by MSVC dumpbin, tied to the executable hash.
- [Real tablebase release-binary report](../validation/windows-x64-portable-tablebases.json):
  136 roots and 1,384 continuations passed against python-chess using the reviewed
  fixture hashes. It is the same corpus described in the previous Phase 6 gate.
- All 23 Companion/JS regression tests passed against this release binary;
  [the platform report](../validation/windows-x64-platform-tests.json) records it.
  The new container-API driver also passed against disposable native processes;
  that verifies driver behavior, not Docker or Linux.
- The included standalone source rebuilt successfully with `--offline --locked`
  for `x86_64-pc-windows-msvc`, using the bundled static-CRT Cargo configuration.

## Run on a clean Windows VM

Copy the portable ZIP, its `.sha256` sidecar, and
`scripts/validate-companion-windows.ps1` to a fresh Windows VM. The runner needs
only inbox Windows PowerShell; Node, Python, Rust and Visual Studio are not
required to run it. It creates and removes its own exact temporary directory.

```powershell
powershell.exe -NoProfile -NonInteractive -File .\validate-companion-windows.ps1 `
  -ZipPath .\ironwood-companion-0.1.0-windows-x64.zip `
  -ReportPath .\windows-clean-report.json `
  -EnvironmentKind clean-vm `
  -EnvironmentEvidence "Fresh Windows 11 x64 VM; record image/build identifier"
```

`EnvironmentKind` defaults to `development`. Clean VM/sandbox labels require an
explicit environment description and are operator assertions, not automatic
proof of OS cleanliness. Retain the base-image identifier, OS build, result and
package hash. A hosted CI Windows runner with developer tools preinstalled is
not a clean-machine substitute. Windows Sandbox is unsupported on Windows Home.
The runner does not change execution policy, Windows features or system settings.

## Linux Docker / QNAP Container Station

`scripts/validate-companion-docker.sh` needs a Linux **x64** Docker daemon and its
CLI, plus ordinary shell utilities. ARM64 is deliberately not claimed by this
gate. It does not require Node, Python or Rust installed on the NAS. It builds
an official Rust/Bookworm production image plus a separate Node/Bookworm
validation toolbox. Building needs internet access, working Docker image pulls,
and adequate spare NAS disk/RAM; Cargo defaults to two build jobs. Check NAS
capacity and architecture before starting a build on shared storage hardware.

The toolbox uses only the already-reviewed MIT `ws` 8.21.0 package, pinned by
npm lock integrity. Node/image tooling has MIT licensing; Debian components
retain their separate licenses. No toolbox is published by this workflow.
Release image digest pinning and OS package/notices audit remain later gates.

The runner:

1. Runs Rust server/protocol unit tests in the builder with networking disabled.
2. Runs the full native/client Node suite against the exact Linux executable in
   the toolbox, also with networking disabled and serialized test files.
3. Creates its own named volume and internal Docker network. It publishes **no
   ports** and mounts no Docker socket or host directories into the containers.
4. Starts the actual production image as UID 10001, read-only root filesystem,
   no capabilities, no-new-privileges, 64 PIDs and 256 MiB memory. It checks Docker
   health state, native healthcheck and private data/token file modes.
5. Seeds only the disposable volume with an original shell UCI fixture and empty
   tablebase directory. It checks authenticated HTTP/WebSocket services, import,
   indexing, local bare-king probing and UCI handshake/search/close.
6. Uses Docker stop/SIGTERM, verifies exit zero/no OOM/no shutdown deadline, then
   removes/replaces the container while retaining the volume and verifies archive
   and explorer persistence.
7. Saves reports, logs, runtime settings, image ID and Docker version under
   `tmp/iwcheck-*/`, and removes only its own test containers/network/volume.
   Tagged validation images and Docker build cache remain available for review.

This does not mount the real tablebase fixture set into Docker, certify an
external real engine binary, or measure large-dataset behavior. Those remain
separate gates. Production NAS deployment, LAN access and shared archive storage
are outside this test run.

## Transfer the current working-tree context

These Companion changes are still uncommitted. A clone of the remote default
branch does not contain this working tree. Prepare the small source/test context:

```powershell
.\scripts\package-companion-validation-context.ps1 `
  -OutputDirectory output/companion-docker-validation-new
```

It includes selected source/tests and minimal web client dependencies, plus a
SHA-256 sidecar. It excludes credentials, owner configs, databases, downloaded
tablebase files, toolchains and `node_modules`. `.dockerignore` separately
allowlists the Docker build inputs. Copy the archive and sidecar to a dedicated
owner-writable NAS test folder; after verifying the checksum there:

```sh
mkdir context
cd context
tar -xzf ../ironwood-companion-validation-context.tar.gz
sh scripts/validate-companion-docker.sh
```

Transfer/access commands depend on a successful owner-authorized SSH login and
a dedicated writable NAS test folder.
No SSH service, credential, permission or firewall change is performed by these
scripts. A Container Station GUI showing a running container alone does not
satisfy the test suite; retain the runner's reports from the actual NAS run.
