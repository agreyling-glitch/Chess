#!/bin/sh
# Run on a Linux x64 Docker host. No host ports or privileged mounts are used.
set -eu
repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo"
command -v docker >/dev/null || { echo 'Docker CLI is required' >&2; exit 1; }
[ "$(docker info --format '{{.OSType}}')" = linux ] || { echo 'Linux Docker is required' >&2; exit 1; }
arch=$(docker info --format '{{.Architecture}}')
case "$arch" in x86_64|amd64) ;; *) echo "This validation gate targets x64; found $arch" >&2; exit 1;; esac
run="iwcheck-$(date +%s)-$$"
image="ironwood-companion-validation:$run"
tools="ironwood-companion-validation-tools:$run"
builder="ironwood-companion-validation-builder:$run"
unit="$run-unit"
volume="$run-data"; network="$run-net"; service="$run-service"; native="$run-native"; exercise="$run-exercise"; persisted="$run-persisted"
mkdir -p tmp
out="$repo/tmp/$run"; mkdir "$out"
cleanup(){
 status=$?
 if [ "$status" -ne 0 ]; then
  docker logs "$service" > "$out/container-failure.log" 2>&1 || true
  printf '{"format":"ironwood-docker-validation","version":1,"status":"failed","exitCode":%s}\n' "$status" > "$out/result.json"
 fi
 for container in "$service" "$native" "$exercise" "$persisted" "$unit"; do docker rm -f "$container" >/dev/null 2>&1 || true; done
 docker network rm "$network" >/dev/null 2>&1 || true
 docker volume rm "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
printf '%s\n' 'Building runtime and development validation images...'
docker build --build-arg CARGO_BUILD_JOBS=2 -t "$image" -f deploy/ironwood-server/Dockerfile .
docker build --target build --build-arg CARGO_BUILD_JOBS=2 -t "$builder" -f deploy/ironwood-server/Dockerfile .
docker run --name "$unit" --network none "$builder" cargo test --locked --offline -p ironwood-server -p ironwood-protocol > "$out/native-unit-tests.log" 2>&1
docker build --build-arg "COMPANION_IMAGE=$image" -t "$tools" -f deploy/ironwood-server/validation.Dockerfile .
native_status=0
docker run --name "$native" --network none "$tools" || native_status=$?
docker cp "$native:/validation/tmp/platform-report.json" "$out/platform-report.json"
docker cp "$native:/validation/tmp/platform-tests.tap" "$out/platform-tests.tap"
[ "$native_status" = 0 ] || exit "$native_status"
docker volume create --label "ironwood.validation=$run" "$volume" >/dev/null
docker network create --internal --label "ironwood.validation=$run" "$network" >/dev/null
start(){
 docker run -d --name "$service" --network "$network" --network-alias companion --label "ironwood.validation=$run" --read-only --cap-drop ALL --security-opt no-new-privileges --pids-limit 64 --memory 256m --mount "type=volume,src=$volume,dst=/data" "$image" >/dev/null
 ready=false
 for n in $(seq 1 60); do if [ "$(docker inspect --format '{{.State.Health.Status}}' "$service")" = healthy ] && docker exec "$service" ironwood-server healthcheck >/dev/null 2>&1; then ready=true; break; fi; sleep 1; done
 [ "$ready" = true ] || { docker logs "$service" >&2; return 1; }
 [ "$(docker exec "$service" id -u)" = 10001 ]
 [ "$(docker exec "$service" stat -c %a /data)" = 700 ]
 [ "$(docker exec "$service" stat -c %a /data/owner-token)" = 600 ]
}
stop(){
 docker stop -t 10 "$service" >/dev/null
 [ "$(docker inspect --format '{{.State.ExitCode}}' "$service")" = 0 ]
 [ "$(docker inspect --format '{{.State.OOMKilled}}' "$service")" = false ]
 docker logs "$service" > "$out/container-$1.log" 2>&1
 grep '"event":"stopped"' "$out/container-$1.log" >/dev/null
 if grep 'shutdown_deadline' "$out/container-$1.log" >/dev/null; then return 1; fi
 docker inspect "$service" --format '{"user":"{{.Config.User}}","readOnly":{{.HostConfig.ReadonlyRootfs}},"memory":{{.HostConfig.Memory}},"pids":{{.HostConfig.PidsLimit}},"ports":{{json .HostConfig.PortBindings}},"exitCode":{{.State.ExitCode}},"oomKilled":{{.State.OOMKilled}}}' > "$out/runtime-settings-$1.json"
 docker rm "$service" >/dev/null
}
start
# Initialization runs as the actual image user and establishes private ownership.
stop initialized
docker run --rm --network none --mount "type=volume,src=$volume,dst=/data" "$tools" node scripts/validate-companion-container-api.mjs seed
start
exercise_status=0
docker run --name "$exercise" --network "$network" --mount "type=volume,src=$volume,dst=/data,readonly" "$tools" node scripts/validate-companion-container-api.mjs exercise || exercise_status=$?
docker cp "$exercise:/validation/tmp/container-exercise-report.json" "$out/container-exercise-report.json"
[ "$exercise_status" = 0 ] || exit "$exercise_status"
stop exercised
start
persisted_status=0
docker run --name "$persisted" --network "$network" --mount "type=volume,src=$volume,dst=/data,readonly" "$tools" node scripts/validate-companion-container-api.mjs persisted || persisted_status=$?
docker cp "$persisted:/validation/tmp/container-persisted-report.json" "$out/container-persisted-report.json"
[ "$persisted_status" = 0 ] || exit "$persisted_status"
stop persisted
docker image inspect "$image" --format '{{.Id}}' > "$out/image-id.txt"
docker version --format '{{json .Server}}' > "$out/docker-server.json"
printf '{"format":"ironwood-docker-validation","version":1,"status":"passed","architecture":"%s","portsPublished":false}\n' "$arch" > "$out/result.json"
printf 'Docker validation passed. Evidence: %s\n' "$out"
