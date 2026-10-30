#!/bin/sh
# Run from the uploaded source directory. Existing volumes are never removed.
set -eu
umask 077
cd "$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
if ! command -v docker >/dev/null 2>&1; then
 station=$(/sbin/getcfg container-station Install_Path -f /etc/config/qpkg.conf)
 PATH="$station/bin:$PATH"; export PATH
fi
command -v docker >/dev/null 2>&1 || { echo 'Start Container Station first.' >&2; exit 1; }
[ "$(id -u)" = 0 ] || { echo 'Use the QNAP administrator account with Docker access.' >&2; exit 1; }
ids=$(docker ps -aq --filter label=com.docker.compose.project=ironwood-companion --filter label=com.docker.compose.service=companion)
set -- $ids
[ "$#" = 1 ] || { echo 'Expected exactly one existing Companion container; nothing changed.' >&2; exit 1; }
container=$1
[ "$(docker inspect --format '{{.State.Running}}' "$container")" = true ] || { echo 'Existing Companion must be running before this upgrade.' >&2; exit 1; }
old_image=$(docker inspect --format '{{.Image}}' "$container")
old_tag=$(docker inspect --format '{{.Config.Image}}' "$container")
case "$old_tag" in ironwood-companion:*) ;; *) echo 'Unexpected existing image; nothing changed.' >&2; exit 1;; esac
project=ironwood-companion
compose_files=$(docker inspect --format '{{index .Config.Labels "com.docker.compose.project.config_files"}}' "$container")
# Container Station supplies its application YAML plus a resource override file.
# Keep the exact file order; relative paths still resolve at the original files.
compose_reversed=$(printf '%s\n' "$compose_files" | awk -F, '{for(i=NF;i>=1;i--) print $i}')
compose_file=
image_entries=0
saved_ifs=$IFS
IFS=,
for file in $compose_files; do
 case "$file" in /*) ;; *) echo 'Cannot identify absolute Compose file paths; nothing changed.' >&2; exit 1;; esac
 [ -f "$file" ] && [ ! -L "$file" ] || { echo 'A Compose file is missing or a symlink; nothing changed.' >&2; exit 1; }
 count=$(grep -Ec '^[[:space:]]*image:.*ironwood-companion:' "$file" || true)
 if [ "$count" -gt 0 ]; then compose_file=$file; image_entries=$((image_entries+count)); fi
done
IFS=$saved_ifs
[ "$image_entries" = 1 ] || { echo 'Cannot safely identify the one Companion image entry across Compose files; nothing changed.' >&2; exit 1; }
if docker compose version >/dev/null 2>&1; then
 compose_flavor=plugin
elif command -v docker-compose >/dev/null 2>&1; then
 compose_flavor=standalone
else echo 'Docker Compose is unavailable; nothing changed.' >&2; exit 1
fi
compose() {
 saved_ifs=$IFS
 IFS='
'
 for file in $compose_reversed; do set -- -f "$file" "$@"; done
 IFS=$saved_ifs
 if [ "$compose_flavor" = plugin ]; then docker compose -p "$project" "$@"; else docker-compose -p "$project" "$@"; fi
}
echo 'Preserving these Container Station Compose files:'
printf '%s\n' "$compose_files"
recovery="$PWD/upgrade-recovery-$(date +%Y%m%d-%H%M%S)-$$"
mkdir -m 700 "$recovery"
docker inspect "$container" > "$recovery/container-before.json"
cp -p "$compose_file" "$recovery/compose-before.yml"
printf '%s\n' "$compose_files" > "$recovery/compose-files.txt"
index=0
saved_ifs=$IFS
IFS=,
for file in $compose_files; do index=$((index+1)); cp -p "$file" "$recovery/compose-input-$index.yml"; done
IFS=$saved_ifs
docker inspect --format '{{range .Mounts}}{{.Type}}|{{.Source}}|{{.Destination}}|{{.RW}}{{println}}{{end}}' "$container" | sort > "$recovery/mounts-before.txt"
[ "$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/data"}}{{.RW}}{{end}}{{end}}' "$container")" = true ] || { echo 'Existing writable /data mount not found; nothing changed.' >&2; exit 1; }
echo 'Existing Companion mounts (these will be preserved):'
cat "$recovery/mounts-before.txt"
docker exec "$container" sha256sum /data/config.json /data/owner-token > "$recovery/settings-before.sha256"
version=$(sed -n 's/^version = "\([^"]*\)".*/\1/p' server/Cargo.toml | head -n 1)
case "$version" in *[!0-9.]*|'') echo 'Invalid Companion source version.' >&2; exit 1;; esac
image=ironwood-companion:qnap-$version
export IRONWOOD_QNAP_IMAGE="$image"
echo 'Building and testing the new image. The existing Companion stays running.'
sh deploy/ironwood-server/qnap-build.sh
[ "$(docker inspect --format '{{.Image}}' "$container")" = "$old_image" ] || { echo 'Existing container changed during build. Nothing upgraded.' >&2; exit 1; }
# Require enough room for an uncompressed equivalent of /data plus a margin.
data_kib=$(docker exec "$container" du -sk /data | awk '{print $1}')
free_kib=$(df -Pk "$recovery" | awk 'NR==2 {print $4}')
case "$data_kib:$free_kib" in *[!0-9:]*|:*) echo 'Cannot verify recovery space; nothing upgraded.' >&2; exit 1;; esac
[ "$free_kib" -gt "$((data_kib + data_kib / 10 + 524288))" ] || { echo 'Not enough free NAS space for a recovery copy. Existing Companion stays running.' >&2; exit 1; }
rollback="ironwood-companion:qnap-rollback-$(date +%Y%m%d-%H%M%S)"
docker tag "$old_image" "$rollback"
printf '%s\n' "$rollback" > "$recovery/previous-image.txt"
started=0
finished=0
cleanup() {
 result=$?
 if [ "$started" = 1 ] && [ "$finished" != 1 ]; then
   echo "Upgrade did not finish. Recovery files: $recovery" >&2
   echo 'No volumes were deleted. Do not restore over the existing archive or remove its data volume.' >&2
 fi
 exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
started=1
echo 'Stopping only Companion for a consistent recovery copy of all /data mounts...'
docker stop --time 120 "$container" >/dev/null
[ "$(docker inspect --format '{{.State.ExitCode}}' "$container")" != 137 ] || { echo 'Companion exceeded its shutdown timeout. Restart the existing container and retry after imports stop.' >&2; exit 1; }
# This includes config, token, games, engines, and nested Import/Reports mounts.
# The private archive stays on the NAS and is never sent through the browser.
if ! docker run --rm --network none --read-only --cap-drop ALL --security-opt no-new-privileges --volumes-from "$container:ro" --entrypoint tar "$image" -C /data -czf - . > "$recovery/data-before-upgrade.tar.gz"; then
 echo 'Recovery copy failed. Restarting unchanged Companion.' >&2
 docker start "$container" >/dev/null
 exit 1
fi
gzip -t "$recovery/data-before-upgrade.tar.gz"
sha256sum "$recovery/data-before-upgrade.tar.gz" > "$recovery/data-before-upgrade.tar.gz.sha256"
# Only change the image field; keep every existing mount, port and setting.
candidate="$compose_file.ironwood-upgrade-$$"
awk -v image="$image" '/^[[:space:]]*image:.*ironwood-companion:/ {match($0,/^[[:space:]]*/);print substr($0,1,RLENGTH) "image: " image;next}{print}' "$compose_file" > "$candidate"
chmod 600 "$candidate"
cp "$candidate" "$recovery/compose-after.yml"
mv "$candidate" "$compose_file"
compose config --quiet
compose up -d --no-deps --force-recreate companion
new_ids=$(docker ps -aq --filter label=com.docker.compose.project=ironwood-companion --filter label=com.docker.compose.service=companion)
set -- $new_ids
[ "$#" = 1 ] || { echo 'Unexpected recreated containers. Review Container Station before proceeding.' >&2; exit 1; }
current=$1
docker inspect --format '{{range .Mounts}}{{.Type}}|{{.Source}}|{{.Destination}}|{{.RW}}{{println}}{{end}}' "$current" | sort > "$recovery/mounts-after.txt"
if ! cmp -s "$recovery/mounts-before.txt" "$recovery/mounts-after.txt"; then
 docker stop --time 120 "$current" >/dev/null
 echo 'Mount verification failed. New Companion stopped; retained recovery copy and previous image.' >&2
 exit 1
fi
i=0
until docker exec "$current" /usr/local/bin/ironwood-server healthcheck >/dev/null 2>&1; do
 i=$((i+1)); [ "$i" -lt 60 ] || { docker stop --time 120 "$current" >/dev/null; echo 'Health check failed. Companion stopped; recovery files retained.' >&2; exit 1; }
 sleep 2
done
docker exec "$current" sha256sum /data/config.json /data/owner-token > "$recovery/settings-after.sha256"
if ! cmp -s "$recovery/settings-before.sha256" "$recovery/settings-after.sha256"; then
 docker stop --time 120 "$current" >/dev/null
 echo 'Config or token changed unexpectedly. Companion stopped for review; recovery copy retained.' >&2
 exit 1
fi
[ "$(docker inspect --format '{{.Image}}' "$current")" = "$(docker image inspect --format '{{.Id}}' "$image")" ]
finished=1
printf '\nUPGRADE COMPLETE: Companion %s is healthy.\n' "$version"
printf 'All mounts, configuration and token verified unchanged.\nRecovery folder (private): %s\nPrevious image retained: %s\n' "$recovery" "$rollback"
echo 'Reconnect the QNAP in Ironwood. Configure its backup folder using a path visible inside the container.'
echo 'The default backup folder is within /data, so it already uses your preserved persistent volume.'
