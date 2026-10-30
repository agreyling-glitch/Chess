#!/bin/sh
# Private preview: build and smoke-test in a dedicated NAS source directory.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)"
case "$(uname -m)" in x86_64|amd64) ;; *) echo 'This package targets the TS-677/Linux x64.' >&2; exit 1;; esac
if ! command -v docker >/dev/null 2>&1; then
  station=$(/sbin/getcfg container-station Install_Path -f /etc/config/qpkg.conf)
  PATH="$station/bin:$PATH"; export PATH
fi
command -v docker >/dev/null 2>&1 || { echo 'Start Container Station first.' >&2; exit 1; }
docker info >/dev/null
printf '\nNAS resources (building may take several minutes):\n'
df -h .
free -m || true
image=${IRONWOOD_QNAP_IMAGE:-ironwood-companion:qnap-preview}
run="ironwood-qnap-check-$(date +%s)-$$"
volume="$run-data"
cleanup() {
  docker rm -f "$run" >/dev/null 2>&1 || true
  docker volume rm "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
echo 'Building Linux x64 Companion. Detailed output is in build.log.'
if ! docker build --platform linux/amd64 --build-arg CARGO_BUILD_JOBS=2 -t "$image" -f deploy/ironwood-server/Dockerfile . >build.log 2>&1; then
  tail -n 60 build.log; exit 1
fi
echo 'Testing the Linux archive backup implementation in the matching build image...'
docker build --platform linux/amd64 --target build --build-arg CARGO_BUILD_JOBS=2 -t "$image-builder" -f deploy/ironwood-server/Dockerfile . >>build.log 2>&1
docker run --rm --network none "$image-builder" cargo test --locked --offline --release -p ironwood-server archive_backups --lib >>build.log 2>&1
docker run --rm --network none "$image-builder" cargo test --locked --offline --release -p ironwood-server archive_library --lib >>build.log 2>&1

docker volume create "$volume" >/dev/null
docker run -d --name "$run" --network none --read-only --cap-drop ALL --security-opt no-new-privileges --pids-limit 128 --memory 512m -v "$volume:/data" "$image" >/dev/null
healthy() {
  i=0
  until docker exec "$run" /usr/local/bin/ironwood-server healthcheck >/dev/null 2>&1; do
    i=$((i+1)); if [ "$i" -ge 30 ]; then docker logs "$run"; return 1; fi
    sleep 2
  done
}
healthy
before=$(docker exec "$run" sha256sum /data/owner-token)
docker restart "$run" >/dev/null
healthy
after=$(docker exec "$run" sha256sum /data/owner-token)
[ "$before" = "$after" ] || { echo 'Credential persistence check failed.' >&2; exit 1; }
docker image inspect "$image" >image-info.json
echo 'Container health and restart persistence passed. Exporting image...'
docker save "$image" | gzip >ironwood-companion-qnap-linux-amd64.tar.gz
sha256sum ironwood-companion-qnap-linux-amd64.tar.gz >ironwood-companion-qnap-linux-amd64.tar.gz.sha256
echo 'Ready: ironwood-companion-qnap-linux-amd64.tar.gz'
echo "The image is already on this NAS: $image."
echo 'Use Applications > Create with compose.qnap.yaml. The exported archive is a backup image.'
