#!/bin/sh
# Private preview: preserve queue/report files before adding the two bind mounts.
set -eu
if ! command -v docker >/dev/null 2>&1; then
  station=$(/sbin/getcfg container-station Install_Path -f /etc/config/qpkg.conf)
  PATH="$station/bin:$PATH"; export PATH
fi
[ "$(id -u)" = 0 ] || { echo 'Run this setup with the QNAP admin account.' >&2; exit 1; }
share=$(readlink -f /share/Container)
[ -d "$share" ] || { echo 'The Container shared folder is missing.' >&2; exit 1; }
base="$share/Ironwood"
for path in "$base" "$base/Import" "$base/Reports"; do
  [ ! -L "$path" ] || { echo "Refusing symbolic-link destination: $path" >&2; exit 1; }
done
ids=$(docker ps -aq --filter label=com.docker.compose.project=ironwood-companion --filter label=com.docker.compose.service=companion)
set -- $ids
[ "$#" = 1 ] || { echo 'Expected exactly one Companion container in the ironwood-companion application.' >&2; exit 1; }
container=$1
image=$(docker inspect --format '{{.Config.Image}}' "$container")
case "$image" in ironwood-companion:*) ;; *) echo 'The selected container is not an Ironwood Companion image.' >&2; exit 1;; esac
# Never copy over existing destination data. Repeated runs fail without stopping.
for name in Import Reports; do
  if [ -d "$base/$name" ] && [ -n "$(ls -A "$base/$name")" ]; then
    echo "$base/$name is not empty. Existing files were left unchanged; review before migration." >&2; exit 1
  fi
done
mkdir -p "$base/Import" "$base/Reports"
echo 'Stopping only Ironwood Companion to copy a consistent Import and Reports snapshot...'
docker stop --time 30 "$container" >/dev/null
docker cp -a "$container:/data/Import/." "$base/Import/"
docker cp -a "$container:/data/Reports/." "$base/Reports/"
[ -z "$(find "$base/Import" "$base/Reports" -type l -print)" ] || { echo 'Symbolic links found; ownership left unchanged.' >&2; exit 1; }
# Container runs as UID/GID 10001. Limit permission changes to these two folders.
chown -R 10001:10001 "$base/Import" "$base/Reports"
chmod -R u+rwX "$base/Import" "$base/Reports"
chmod 750 "$base/Import" "$base/Reports"
# Check access as the actual service user, including NAS ACLs, before proceeding.
docker run --rm --network none --read-only --cap-drop ALL --user 10001:10001 \
  --entrypoint /bin/sh -v "$base/Import:/shared-import" -v "$base/Reports:/shared-reports" \
  "$image" -c 'test -r /shared-import && test -w /shared-import && test -x /shared-import && test -w /shared-reports && test -x /shared-reports'
echo 'Folders prepared: Container/Ironwood/Import and Container/Ironwood/Reports.'
echo 'Original files remain in the private data volume. No archive or token was copied.'
echo 'Companion is stopped. Now recreate the existing application using the updated compose.qnap.yaml.'
echo 'Keep the original named volume at /data, then confirm Companion is healthy.'
