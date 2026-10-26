# QNAP Windows validation VM preparation

Internal release-validation plan. VM creation is pending NAS preflight and Windows
installation-media selection. No VM, virtual network or disk has been created.

Run `scripts/check-qnap-windows-vm.ps1` from the repository in PowerShell. It uses
SSH to admin@192.168.0.2 and prompts for the password; it does not save it. The
NAS script only reads CPU, virtualization availability, RAM, storage and QKVM
package metadata. It does not change running containers or virtual switches.

Confirm Virtualization Station supports the actual NAS CPU and that the selected
Windows release supports that host CPU. A TS-677 model label alone does not
establish Windows 11 eligibility. Do not treat an installation using requirement
bypasses as a supported clean-Windows acceptance baseline.

Proposed dedicated VM once preflight passes:

- Name: Ironwood-Win11-Clean.
- Windows 11 x64 from licensed Microsoft media, or the official 90-day Enterprise
  evaluation under its evaluation terms. Do not copy an activated physical system.
- 4 virtual CPUs, 8 GiB RAM, 100 GiB thin-provisioned disk, subject to current NAS
  capacity and existing VM workloads.
- Windows 11 template with UEFI, Secure Boot and virtual TPM 2.0 as supported by
  the installed Virtualization Station version.
- Use an existing working virtual switch; do not reconfigure the NAS network to
  create the test VM. No public port forwarding or changes to Companion mounts.
- Install current Windows updates and only necessary VM hardware drivers. Record
  any QNAP guest tools. Leave normal Windows security enabled and use a standard
  user for application acceptance. Do not install developer runtimes/tools.
- Shut down and snapshot as Clean-Windows-before-Companion before transferring
  the validation kit. Keep the Windows evaluation expiry in the VM notes.

Copy `output/companion/clean-windows-validation` into the guest and follow its
READ-ME. Enter the actual VM platform, Windows edition/build and snapshot name in
EnvironmentEvidence. Preserve the JSON report and notes on manual GUI/browser,
real engine download/search and import/export checks. Revert to the clean snapshot
for repeatable acceptance of a later package.

References:
- https://www.qnap.com/en-us/how-to/faq/article/what-operating-system-can-i-deploy-in-virtualization-station
- https://www.qnap.com/en-us/how-to/faq/article/how-do-i-create-and-install-a-windows-11-vm-in-virtualization-station
- https://learn.microsoft.com/en-us/windows/whats-new/windows-11-requirements
- https://www.microsoft.com/en-us/evalcenter/evaluate-windows-11-enterprise
