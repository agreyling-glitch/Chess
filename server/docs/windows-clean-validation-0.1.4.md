# Companion 0.1.4 clean Windows validation

Internal validation record, October 25, 2026. No publication or release upload.

## Environment

- Fresh Microsoft Windows 11 Enterprise Evaluation x64, initially build
  26300.9457, updated to 26300.9711 before the final passing run.
- VirtualBox 7.2.22; bundled Guest Additions 7.2.22 installed by the user.
- VM `Ironwood-Win11-Clean`, baseline snapshot `Clean Windows - before Companion`.
- Four CPUs, 8 GiB RAM, 100 GiB dynamic disk, NAT networking.
- USB Tablet pointing device enabled after PS/2 input problems.
- Validation executed without administrator elevation under `ironwoodtest`.
  This is the installer's local administrator account with its normal filtered
  token, not a separate account removed from the Administrators group.
- No Rust, Cargo, Node or Git found on PATH. Windows' Python Store alias exists;
  it is not an installed Python runtime. Server launch uses a restricted PATH.
- No host Companion data, owner token or private release signing key copied.

## Automated result: passed

Report: `output/companion/windows-clean-validation.json`.
The pre-update passing run is retained separately as
`output/companion/windows-clean-validation-before-updates.json`.

ZIP SHA-256:
`0f0b6ac8f96662cf44cb890d4787bec41efdd7c40d5347341ef234fe649c3b6b`.

The run verified package checksum and static CRT metadata, private data
provisioning, health/authentication/origin restrictions, service capabilities,
archive PGN import and binary-storage PGN export, duplicate/short-game handling,
opening indexing and explorer, local tablebase draw, synthetic UCI handshake and
search, restart persistence, token rotation, and graceful shutdown.

The optional `-PublisherPackage weiss-2-0-windows-x64` check also passed:
Companion downloaded Weiss 2.0 from the configured original GitHub publisher,
verified/extracted/registered it, and completed a depth-two UCI search. This
extends the standalone validation harness; the shipped release ZIP is unchanged.

## Windows Update and remaining acceptance

The online scan completed and found five regular software/security updates:
KB5007651, KB890830, KB2267602, KB5127694, and KB5127648. All downloaded during
the elevated retry. The initial unelevated installation attempt failed with
0x80240044 (administrator permission required). The user approved the elevated
retry, which installed all five updates successfully. The VM was restarted
normally. Results are saved in `output/companion/windows-clean-update-install.json`.

The final online scan completed on build 26300.9711 with no restart pending:
`output/companion/windows-clean-update-after-reboot.json`. It offers two additional
staged-channel platform updates: Defender KB4052623 (4.18.26100.4) and Windows
Security KB5007651 (10.0.29680.1000). These were not installed during this run;
do not claim the update list is empty. The exact-package validation, including
the real publisher download and search, passed again after the five updates
and reboot. The final report's environment evidence references that scan.

The portable package was extracted under the guest user's LocalAppData
`Ironwood/CompanionApp` directory. Its native Companion process started, and the
local service health check succeeded. Desktop capture became black while guest
commands continued working; dashboard/browser visual acceptance is unverified.
Do not count GUI interaction, tray exit, browser pairing, browser download
warnings, or the undeployed public update feed as passed.
