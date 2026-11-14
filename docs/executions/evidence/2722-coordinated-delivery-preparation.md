# #2722 actual-relay integration evidence

Executor-authored receipt, 2026-11-14. Starting source d7714be; production unchanged.
Raw local logs live in `/home/bryce/services/releases/`:
`2722-host-discovery-integration-build.log` (initial compile failure),
`2722-host-discovery-integration-build-retry.log` (pass with fixture warning),
`2722-host-discovery-integration-build-final.log` (clean pass),
`2722-host-discovery-integration-run1.log` (one scenario passed).

Cached verification binary SHA-256:

- `target/debug/buzz-host-runtime`: `c01d619c6523446e35b9f36b37d417eff11b1cdf0b89ee12875d3373fe52a56f`
- `target/debug/buzz-acp`: `50393875f45e339fdbe1971fd4b81cfee8590538e07ecbf00440796d36c67b0a`

The host/runtime implementation has not changed since those binaries' prior
accepted seam runs. No delivery artifact was built in this phase.

Reproduction: build the standard integration target offline/locked with `-j2`,
then start the existing `scripts/3720-registration-proof.sh up`. Run the following
task-only invocation from the ordinary assigned UID. It is retained locally as
`/home/bryce/services/releases/2722-run-host-discovery.sh`; it is deliberately not
a new repository runner mode. Replace the compiled integration-binary hash only
if Cargo reports a different filename. No real service/container may substitute
for the exact owned/labeled runner. The outer timeout bounds the process group;
the test additionally bounds its clean-env child to120seconds.

```bash
#!/usr/bin/env bash
# Task-only execution receipt helper; not repository tooling or a general runner.
set -euo pipefail
lane=/home/bryce/services/buzz-wt-3720-host-registration
runner=buzz-3720-registration-runner
pg=buzz-3720-registration-postgres
redis=buzz-3720-registration-redis
for name in "$runner" "$pg" "$redis"; do
  [[ "$(docker inspect -f '{{index .Config.Labels "buzz.proof"}}' "$name")" == 3720 ]]
done
[[ "$(docker inspect -f '{{.HostConfig.NetworkMode}}' "$runner")" == none ]]
runner_pid="$(docker inspect -f '{{.State.Pid}}' "$runner")"
[[ "$runner_pid" =~ ^[1-9][0-9]*$ ]]
[[ "$(id -u)" == 1002 && "$(id -g)" == 1003 ]]
proof_home="$(mktemp -d /tmp/buzz-2722-proof.XXXXXX)"
database="buzz_3720_fixture_${BASHPID}_${RANDOM}"
cleanup() {
  docker exec "$pg" dropdb -U buzz3720 --if-exists --force "$database"
  rm -rf -- "$proof_home"
}
trap cleanup EXIT
docker exec "$pg" createdb -U buzz3720 --template=template0 "$database"
docker exec -w /proof-schema "$pg" psql -U buzz3720 -d "$database" -v ON_ERROR_STOP=1 -f schema.sql > "$lane/target/2722-schema-$database.log" 2>&1
[[ "$(docker inspect -f '{{.State.Pid}}' "$runner")" == "$runner_pid" ]]
[[ "$(docker inspect -f '{{index .Config.Labels "buzz.proof"}}' "$runner")" == 3720 ]]
cd "$proof_home"
sudo -n /usr/bin/timeout --signal=TERM --kill-after=5s 150s /usr/bin/nsenter --target "$runner_pid" --net \
  /usr/bin/setpriv --reuid=1002 --regid=1003 --clear-groups \
  /usr/bin/env -i HOME="$proof_home" PATH=/usr/bin:/bin \
  BUZZ_3720_ISOLATED_FIXTURE=1 \
  BUZZ_TEST_DATABASE_URL="postgres://buzz3720:synthetic3720@127.0.0.1:5432/$database" \
  "$lane/target/debug/deps/host_discovery-68c2206811c617bd" \
  --ignored --exact owner_relay_host_discovery_selected_deploy_and_revoke --nocapture
```

Observed stdout (synthetic identifiers/payloads are not logged):

```text

running 1 test

running 1 test
PASS actual owner -> relay EVENT/REQ -> host ACP -> exact model; zero preCreate effects
PASS exact request/reply replay after host reopen; wrong parent refused
PASS selected Deploy admitted once; actual runtime guard/ACP ran; fake conversation harness READY
PASS cleanup receipt; fixture home/work preserved; revoked query/read/execution refused
test owner_relay_host_discovery_selected_deploy_and_revoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.29s

test owner_relay_host_discovery_selected_deploy_and_revoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.30s

```

After the script exited0, an independent `psql` count of databases named
`buzz_3720_fixture_%` returned0. `scripts/3720-registration-proof.sh down`
removed the three owned containers. `docker ps -a --filter label=buzz.proof=3720
--format '{{.Names}}'` returned no rows. Exact integration process lookup returned
no rows. The runtime fake provider/manager processes are synchronously awaited;
no fixture subprocess is intentionally detached.

The namespace uses host filesystem/tools as ordinary UID1002/GID1003; only the
network namespace is isolated. Fake provider/manager paths and HOME/workspace
are temporary. This is not a container-filesystem sandbox, real login, systemd,
TLS, native UI or conversational response proof.
