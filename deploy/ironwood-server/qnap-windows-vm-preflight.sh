#!/bin/sh
# Read-only QNAP preflight for an isolated Windows validation VM.
set -eu
echo 'NAS model:'
/sbin/getcfg System Model -f /etc/config/uLinux.conf || true
echo 'QTS version:'
/sbin/getcfg System Version -f /etc/config/uLinux.conf || true
echo 'Architecture:'
uname -m
echo 'CPU model:'
awk -F: '/^model name/{sub(/^ +/,"",$2);print $2;exit}' /proc/cpuinfo
echo 'Hardware virtualization CPU flag:'
awk '/^flags/{if($0 ~ / svm / || $0 ~ / vmx /) print "Present";else print "Not reported";exit}' /proc/cpuinfo
echo 'KVM device:'
if [ -c /dev/kvm ]; then ls -ln /dev/kvm; else echo 'Not available (Virtualization Station may not be installed/enabled)'; fi
echo 'Memory (kB):'
awk '/^MemTotal:|^MemAvailable:/{print}' /proc/meminfo
echo 'Data volume space:'
df -h /share/CACHEDEV1_DATA
echo 'Virtualization Station (QKVM) metadata:'
for key in Version Enable Install_Path; do
 printf '%s: ' "$key"
 /sbin/getcfg QKVM "$key" -f /etc/config/qpkg.conf || true
done
echo 'Virtualization-related package sections:'
awk '/^\[/{if(tolower($0) ~ /qkvm|virtualization/)print}' /etc/config/qpkg.conf
echo 'Preflight complete. No NAS configuration or files were changed.'
