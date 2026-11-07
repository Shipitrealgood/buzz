# #3750 WSL setup-agent handoff (prepared; NOT RUN)

Conductor supplies: accepted source SHA, Windows artifact/checksum receipt,
Linux rehearsal archive/checksums, and the Windows-selected **public** owner key.
Use an existing compatible Ubuntu 24.04 WSL session with the required dependencies
already present; a new distro is not required. Create disposable fixture resources:
fresh loopback-only DB/Redis containers, a dedicated unprivileged fixture identity,
private state and synthetic keys. The human should receive one ready local
community URL and the host-console invitation, not setup chores. Reuse no existing
service, data, agent credential/provider home or service account. Make no global
package/config changes; report missing dependencies as a concrete prerequisite.
Do not set `GITHUB_ACTIONS` to bypass the hosted probe's guard. The actual-service
isolation gate ran separately in the disposable GitHub VM; this app journey does
not replace it.

Read `windows-journey.md`, the pinned `registration-systemd-smoke.py` and
`loopback_tls.py`. Verify the archive SHA256SUMS before running a binary. Confirm
Windows localhost reaches WSL loopback; stop if it would require a public bind,
tunnel or unrelated firewall change. Record all subprocess PIDs/container IDs.
The only local listeners for this fixture are 127.0.0.1 ports 35432, 36379,
39552, 39553, 39554 and 39555; refuse conflicts rather than stopping their owners.

## Fresh services and schema

Use the workflow-pinned images. Commands assume `PWD` is the accepted checkout
and the supplied binaries are in `target/debug`; adjust only the artifact path.
Use a fresh task tag; pre-existing names cause a stop, never cleanup/adoption.

```bash
fixture_tag=buzz3750-$(date +%s)
fixture_dir=$(mktemp -d /tmp/buzz3750.XXXXXX)
chmod 700 "$fixture_dir"
docker network create --internal --label buzz.proof=3750 "$fixture_tag"
docker run -d --name "$fixture_tag-pg" --label buzz.proof=3750 --network "$fixture_tag" \
  -p 127.0.0.1:35432:5432 --tmpfs /var/lib/postgresql/data:rw,nosuid,size=1g \
  --mount "type=bind,src=$PWD/schema,dst=/fixture-schema,readonly" \
  -e POSTGRES_DB=fixture -e POSTGRES_USER=fixture -e POSTGRES_PASSWORD=fixture \
  postgres@sha256:3b0cda93be7c8dba2ed484d19393d93845dc3dafb88973e248ba8e6d99d784e1
docker run -d --name "$fixture_tag-redis" --label buzz.proof=3750 --network "$fixture_tag" \
  -p 127.0.0.1:36379:6379 --tmpfs /data:rw,nosuid,size=64m \
  redis@sha256:21090af93f8324db2f0f0f4f38a7c60369d47d917e000c3423d89d2ae2883537 \
  redis-server --save '' --appendonly no
# Wait at most 30 seconds for this exact fresh Postgres container to be ready.
docker exec "$fixture_tag-pg" pg_isready -U fixture
docker exec -w /fixture-schema "$fixture_tag-pg" psql -U fixture -d fixture \
  -v ON_ERROR_STOP=1 -f schema.sql
```

Failure is a fixture prerequisite to resolve, never permission to use another
DB. This desired-state fresh schema is not a deployed migration-lineage gate.

## TLS and relay

Generate the one-day CA/server certificate with the exact OpenSSL commands in
`registration-systemd-smoke.py` (the block from `cert = stage / "ca.crt"` through
the server certificate signing), with `stage` set to `fixture_dir`. This reuses
the existing fixture recipe and `DNS:localhost` SAN. Keep private files 0600.
Generate a fresh secp256k1 relay key using that script's `cryptography` key
creation. Write it only to a private local fixture env file; record its public
x-coordinate for hostd. No key is sent to the conductor/chat or artifact upload.

Start `buzz-relay` with a clean environment (no inherited BUZZ_AUTH_TAG,
BUZZ_PRIVATE_KEY, production URLs or credentials), the explicit settings in
`windows-journey.md`, plus:

```text
DATABASE_URL=postgres://fixture:fixture@127.0.0.1:35432/fixture
REDIS_URL=redis://127.0.0.1:36379/0
RELAY_OWNER_PUBKEY=<Windows-selected public key supplied by conductor>
BUZZ_RELAY_PRIVATE_KEY=<fresh synthetic key, private local environment only>
BUZZ_GIT_CONFORMANCE_PROBE=false
BUZZ_GIT_REPO_PATH=<fixture_dir>/repos
BUZZ_GIT_PACK_CACHE_PATH=<fixture_dir>/pack-cache
BUZZ_GIT_HOOK_HMAC_SECRET=<fresh synthetic secret, private local environment only>
RUST_LOG=warn
```

Run the pinned `loopback_tls.py` command from `windows-journey.md`. Read NIP-11
with the generated public CA and assert v2 advertised/v1 absent. Confirm that
its public relay key matches the generated public coordinate. Copy only the
public CA to Windows and coordinate installation/removal of that exact thumbprint.
Do not reuse the disposable owner-driver secret as the Windows identity.

## Host and ready receipt

Within the approved WSL fixture, create a dedicated `buzz3750-fixture` unprivileged
account (fail if it already exists) with no login shell and a private 0700
`/var/lib/buzz3750-fixture` state directory. Copy the reviewed `buzz-hostd` binary
and public CA to root-owned readable fixture paths; this account must not gain
access to the human's 0700 `fixture_dir` or its synthetic relay private key.
Use `sudo -u buzz3750-fixture` for the exact `registration init`, `registration
run` and TTY-only `registration offer` commands in `windows-journey.md`, replacing
its state/CA placeholders with those fixed fixture paths. Never use `serve`.

Return ready only after: correct binary hashes; fixture membership configured;
NIP-11 key/capability verified over TLS; Windows loopback route verified; hostd
running unprivileged; no pre-existing agents/providers touched. Supply the local
community URL, public CA thumbprint, public host key, recorded PIDs/container IDs,
and an invitation obtained directly from the private host console. The invitation
must not enter the shared handoff or screenshots. Bryce can then perform the
six app observations in `windows-journey.md`.

## Cleanup ownership

Stop the exact recorded hostd, TLS and relay PIDs; wait for exit. Remove only
`$fixture_tag-pg` and `$fixture_tag-redis` after verifying their `buzz.proof=3750`
labels, with their anonymous volumes; remove their exact network. Remove the
private fixture directory, dedicated account/state and root-owned fixture binary
and CA copy. Preserve the existing WSL distro and all unrelated resources. The
Windows operator removes only the fixture community and recorded CA thumbprint,
preserving the shared application profile. Return a cleanup receipt
or a named owner/deadline for intentionally retained fixture resources.
