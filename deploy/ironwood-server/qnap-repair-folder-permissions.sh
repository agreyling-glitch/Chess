#!/bin/sh
# Private preview: repair only the two Companion shared-folder trees.
set -eu
if ! command -v docker >/dev/null 2>&1; then
  station=$(/sbin/getcfg container-station Install_Path -f /etc/config/qpkg.conf)
  PATH="$station/bin:$PATH"; export PATH
fi
[ "$(id -u)" = 0 ] || { echo 'Run with the QNAP admin account.' >&2; exit 1; }
share=$(readlink -f /share/Container)
[ -d "$share" ] || { echo 'Container shared folder missing.' >&2; exit 1; }
base="$share/Ironwood"
for path in "$base" "$base/Import" "$base/Reports"; do
  [ -d "$path" ] && [ ! -L "$path" ] || { echo "Expected real folder: $path" >&2; exit 1; }
done
# Refuse links before recursive ownership changes; never follow a shared-folder link.
[ -z "$(find "$base/Import" "$base/Reports" -type l -print)" ] || { echo 'Symbolic links found; permissions left unchanged.' >&2; exit 1; }
ids=$(docker ps -aq --filter label=com.docker.compose.project=ironwood-companion --filter label=com.docker.compose.service=companion)
set -- $ids
[ "$#" = 1 ] || { echo 'Expected one Ironwood Companion container.' >&2; exit 1; }
container=$1
image=$(docker inspect --format '{{.Config.Image}}' "$container")
case "$image" in ironwood-companion:*) ;; *) echo 'Unexpected image; nothing changed.' >&2; exit 1;; esac
running=$(docker inspect --format '{{.State.Running}}' "$container")
if [ "$running" = true ]; then docker stop --time 30 "$container" >/dev/null; fi
trap 'if [ "$running" = true ]; then docker start "$container" >/dev/null; fi' EXIT
chown -R 10001:10001 "$base/Import" "$base/Reports"
chmod -R u+rwX "$base/Import" "$base/Reports"
docker run --rm --network none --read-only --cap-drop ALL --user 10001:10001 \
  --entrypoint /bin/sh -v "$base/Import:/shared-import" -v "$base/Reports:/shared-reports" \
  "$image" -c 'find /shared-import /shared-reports >/dev/null && test -w /shared-import && test -w /shared-reports'
echo 'Import and Reports permissions repaired. Archive and token files were not changed.'
