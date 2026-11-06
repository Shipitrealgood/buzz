#!/usr/bin/env bash
# Task-only network namespace: all listeners loopback, no external egress,
# no host ports, inherited URLs, production credentials or persistent volumes.
set -euo pipefail
lane="$(cd "$(dirname "$0")/.." && pwd)"
runner=buzz-3720-registration-runner
pg=buzz-3720-registration-postgres
redis=buzz-3720-registration-redis
proxy=buzz-3720-registration-tls-proxy
case "${1:-}" in
up)
  for name in "$runner" "$pg" "$redis"; do
    if docker container inspect "$name" >/dev/null 2>&1; then
      echo "refusing existing task container: $name" >&2; exit 1
    fi
  done
  docker run -d --name "$runner" --label buzz.proof=3720 --network none --tmpfs /var/lib/postgresql/data:rw,nosuid,size=16m \
    --mount "type=bind,src=$lane,dst=$lane" --workdir "$lane" \
    --entrypoint sleep postgres:17-trixie infinity
  docker run -d --name "$pg" --label buzz.proof=3720 --network "container:$runner" \
    --tmpfs /var/lib/postgresql/data:rw,nosuid,size=1g \
    --mount "type=bind,src=$lane/schema,dst=/proof-schema,readonly" \
    -e POSTGRES_USER=buzz3720 -e POSTGRES_PASSWORD=synthetic3720 -e POSTGRES_DB=postgres \
    postgres:17-trixie -c listen_addresses=127.0.0.1
  docker run -d --name "$redis" --label buzz.proof=3720 --network "container:$runner" \
    --tmpfs /data:rw,nosuid,size=64m redis:alpine redis-server --bind 127.0.0.1 --save '' --appendonly no
  for _ in {1..30}; do
    if docker exec "$pg" pg_isready -h127.0.0.1 -U buzz3720 >/dev/null 2>&1; then exit 0; fi
    sleep 1
  done
  echo 'fixture Postgres did not become ready' >&2; exit 1
  ;;
tls)
  [[ "$(docker inspect -f '{{index .Config.Labels "buzz.proof"}}' "$runner")" == 3720 ]] || exit 1
  if docker container inspect "$proxy" >/dev/null 2>&1; then echo 'refusing existing TLS proxy' >&2; exit 1; fi
  mkdir -p "$lane/target/3720-tls"
  openssl req -x509 -newkey rsa:2048 -nodes -sha256 -days 2 -subj /CN=buzz3720-fixture-ca \
    -keyout "$lane/target/3720-tls/ca.key" -out "$lane/target/3720-tls/ca.pem" >/dev/null 2>&1
  openssl req -newkey rsa:2048 -nodes -sha256 -subj /CN=registry-tls-a.test \
    -keyout "$lane/target/3720-tls/server.key" -out "$lane/target/3720-tls/server.csr" >/dev/null 2>&1
  cat > "$lane/target/3720-tls/extensions" <<'EXT'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:registry-tls-a.test,DNS:registry-tls-b.test
EXT
  openssl x509 -req -in "$lane/target/3720-tls/server.csr" -CA "$lane/target/3720-tls/ca.pem" \
    -CAkey "$lane/target/3720-tls/ca.key" -CAcreateserial -out "$lane/target/3720-tls/server.pem" \
    -days 2 -sha256 -extfile "$lane/target/3720-tls/extensions" >/dev/null 2>&1
  cat > "$lane/target/3720-tls/nginx.conf" <<'NGINX'
events {}
http {
  access_log off;
  server {
    listen 127.0.0.1:34443 ssl;
    server_name registry-tls-a.test registry-tls-b.test;
    ssl_certificate /proof-tls/server.pem;
    ssl_certificate_key /proof-tls/server.key;
    location / {
      proxy_pass http://127.0.0.1:32000;
      proxy_http_version 1.1;
      proxy_set_header Host $http_host;
      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection "upgrade";
    }
  }
}
NGINX
  docker exec -u0 "$runner" sh -c 'echo "127.0.0.1 registry-tls-a.test registry-tls-b.test" >> /etc/hosts'
  docker run -d --name "$proxy" --label buzz.proof=3720 --network "container:$runner" \
    --mount "type=bind,src=$lane/target/3720-tls,dst=/proof-tls,readonly" \
    --entrypoint nginx nginx:stable-bookworm -c /proof-tls/nginx.conf -g 'daemon off;'
  ;;
run)
  : "${2:?supply exact compiled test binary relative to lane}"
  shift; binary="$1"; shift
  [[ "$binary" == target/* && -x "$lane/$binary" ]] || { echo 'unsafe test binary' >&2; exit 1; }
  # Per-process database is newly created from desired state and dropped on exit.
  database="buzz_3720_fixture_${BASHPID}_${RANDOM}"
  [[ "$database" =~ ^buzz_3720_fixture_[0-9]+_[0-9]+$ ]] || exit 1
  for name in "$runner" "$pg" "$redis"; do
    [[ "$(docker inspect -f '{{index .Config.Labels "buzz.proof"}}' "$name")" == 3720 ]] || exit 1
  done
  cleanup() { docker exec "$pg" dropdb -U buzz3720 --if-exists --force "$database"; }
  trap cleanup EXIT
  docker exec "$pg" createdb -U buzz3720 --template=template0 "$database"
  docker exec -w /proof-schema "$pg" psql -U buzz3720 -d "$database" -v ON_ERROR_STOP=1 -f schema.sql > "$lane/target/3720-schema-$database.log" 2>&1
  docker exec -u "$(id -u):$(id -g)" \
    -e "BUZZ_TEST_DATABASE_URL=postgres://buzz3720:synthetic3720@127.0.0.1:5432/$database" \
    -e BUZZ_TEST_SCHEMA_MODE=desired -e REDIS_URL=redis://127.0.0.1:6379/0 \
    -e TMPDIR=/tmp -e BUZZ_3720_ISOLATED_FIXTURE=1 \
    "$runner" "$lane/$binary" "$@"
  ;;
down)
  for name in "$proxy" "$redis" "$pg" "$runner"; do
    if docker container inspect "$name" >/dev/null 2>&1; then
      [[ "$(docker inspect -f '{{index .Config.Labels "buzz.proof"}}' "$name")" == 3720 ]] || exit 1
      docker rm -fv "$name"
    fi
  done
  ;;
*) echo 'usage: 3720-registration-proof.sh up|tls|run target/debug/deps/TEST [ARGS...]|down' >&2; exit 1;;
esac
