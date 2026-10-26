set -eu
if ! command -v docker >/dev/null 2>&1; then
 station=$(/sbin/getcfg container-station Install_Path -f /etc/config/qpkg.conf)
 PATH="$station/bin:$PATH"; export PATH
fi
ids=$(docker ps -q --filter label=com.docker.compose.project=ironwood-companion --filter label=com.docker.compose.service=companion)
set -- $ids
[ "$#" = 1 ] || { echo 'Expected one running Ironwood Companion.'; exit 1; }
container=$1
docker inspect --format 'Image: {{.Config.Image}} Image ID: {{.Image}} User: {{.Config.User}} Read-only root: {{.HostConfig.ReadonlyRootfs}}' "$container"
docker exec "$container" /bin/sh -c 'id; echo "Data folder permissions:"; ls -ldn /data /data/Import /data/Import/.processing /data/Import/Failed /data/Reports; echo "Archive file permissions:"; find /data -maxdepth 2 -type f -name "*.sqlite*" -exec ls -ln {} \;; echo "Available disk:"; df -h /data; for p in /data /tmp /data/Import /data/Reports; do if test -w "$p"; then echo "Writable: $p"; else echo "Not writable: $p"; fi; done'
echo 'Recent Companion logs:'
docker logs --tail 80 "$container" 2>&1
