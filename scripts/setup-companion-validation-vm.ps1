param(
    [string]$VmName = 'Ironwood-Win11-Clean',
    [string]$BaseFolder = 'D:\VirtualBox VMs',
    [string]$VBoxManage = 'C:\Program Files (x86)\VBoxManage.exe'
)
$ErrorActionPreference = 'Stop'
if (!(Test-Path -LiteralPath $VBoxManage)) { throw 'Set VBoxManage to the installed VirtualBox command-line tool.' }
if ($VmName -notmatch '^[A-Za-z0-9_-]+$') { throw 'Use a simple VM name.' }
$vmBase = [IO.Path]::GetFullPath($BaseFolder)
$vmFolder = Join-Path $vmBase $VmName
if (Test-Path -LiteralPath $vmFolder) { throw 'The VM folder already exists; no files were changed.' }
$registered = & $VBoxManage list vms
if ($LASTEXITCODE -ne 0) { throw 'Could not read registered VMs.' }
if ($registered | Where-Object { $_.StartsWith('"' + $VmName + '" ') }) { throw 'This VM is already registered; no settings were changed.' }
function VBox([string[]]$VmArguments) {
    & $VBoxManage @VmArguments
    if ($LASTEXITCODE -ne 0) { throw "VirtualBox failed: $($VmArguments[0]). The partial VM was retained for inspection." }
}
VBox @('createvm','--name',$VmName,'--ostype','Windows11_64','--basefolder',$vmBase,'--register')
VBox @('modifyvm',$VmName,'--memory=8192','--cpus=4','--cpu-profile=host','--ioapic=on','--firmware=efi64','--tpm-type=2.0','--graphicscontroller=vboxsvga','--vram=128','--accelerate-3d=off','--nic1=nat','--nic-type1=82540EM','--cable-connected1=on','--clipboard-mode=disabled','--drag-and-drop=disabled','--audio-enabled=off','--usb-ohci=off','--usb-ehci=off','--usb-xhci=off','--vrde=off','--boot1=dvd','--boot2=disk','--boot3=none','--boot4=none','--description','Clean Windows 11 x64 validation for Ironwood Companion. Use licensed/evaluation Microsoft media. Snapshot before installing Companion. No developer tools.')
VBox @('modifynvram',$VmName,'inituefivarstore')
VBox @('modifynvram',$VmName,'enrollmssignatures')
VBox @('modifynvram',$VmName,'enrollorclpk')
VBox @('modifynvram',$VmName,'secureboot','--enable')
VBox @('setextradata',$VmName,'VBoxInternal2/EfiGraphicsResolution','1600x900')
VBox @('modifyvm',$VmName,'--mouse=usbtablet','--usb-ohci=on')
VBox @('storagectl',$VmName,'--name','SATA','--add','sata','--controller','IntelAhci','--portcount','2','--bootable','on')
$disk = Join-Path $vmFolder ($VmName + '.vdi')
VBox @('createmedium','disk','--filename',$disk,'--size','102400','--format','VDI','--variant','Standard')
VBox @('storageattach',$VmName,'--storagectl','SATA','--port','0','--device','0','--type','hdd','--medium',$disk)
VBox @('storageattach',$VmName,'--storagectl','SATA','--port','1','--device','0','--type','dvddrive','--medium','emptydrive')
Write-Output "Configured $VmName at $vmFolder. Attach the Microsoft evaluation ISO before starting."
