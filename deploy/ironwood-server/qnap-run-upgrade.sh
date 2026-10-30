#!/bin/sh
# Keep a long NAS build alive if the SSH client disconnects; never start it twice.
set -eu
umask 077
cd "$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
if [ "${1:-}" = --worker ]; then
 result=0
 sh deploy/ironwood-server/qnap-upgrade.sh || result=$?
 printf '%s\n' "$result" > upgrade-result.tmp
 mv upgrade-result.tmp upgrade-result
 rm -f .upgrade-running/pid
 rmdir .upgrade-running
 exit "$result"
fi
if [ -f upgrade-result ]; then
 echo 'A previous run has completed. Retaining its result; review before starting another upgrade.'
 cat upgrade-run.log
 exit "$(cat upgrade-result)"
fi
if [ -f .upgrade-running/pid ] && ! kill -0 "$(cat .upgrade-running/pid)" 2>/dev/null; then
 mv .upgrade-running ".upgrade-stale-$(date +%Y%m%d-%H%M%S)-$$"
fi
if mkdir -m 700 .upgrade-running 2>/dev/null; then
 (trap '' HUP; exec sh deploy/ironwood-server/qnap-run-upgrade.sh --worker) > upgrade-run.log 2>&1 < /dev/null &
 printf '%s\n' "$!" > .upgrade-running/pid
 echo 'NAS upgrade job started. It continues safely if this SSH window disconnects.'
else
 echo 'Attaching to the existing NAS upgrade job; no second upgrade will be started.'
fi
line=1
while :; do
 if [ -f upgrade-run.log ]; then
  count=$(wc -l < upgrade-run.log | tr -d ' ')
  if [ "$count" -ge "$line" ]; then sed -n "${line},${count}p" upgrade-run.log; line=$((count+1)); fi
 fi
 if [ -f upgrade-result ]; then exit "$(cat upgrade-result)"; fi
 if [ -f .upgrade-running/pid ] && ! kill -0 "$(cat .upgrade-running/pid)" 2>/dev/null; then
  echo 'NAS job stopped unexpectedly. Preserve recovery files and inspect before retrying.' >&2
  exit 1
 fi
 sleep 5
done
