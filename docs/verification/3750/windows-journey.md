# #3750 native Windows journey — operator gate (NOT RUN)

The conductor must first pin an independently reviewed commit and the successful
Windows Canary and Linux registration rehearsal artifacts. The local Debian
executor cannot run this gate. A Windows unit-test pass or browser mock is not
this journey. Use the fork's existing `build/paired-hosts-canary` workflow and
`verify/paired-hosts-staging` workflow; both retain their fork/ref allowlists.
No installer publication, production relay, public tunnel, provider credentials,
agent creation, or live conversations are part of this test.

## Required fixture

- A Windows machine running the exact canary installer, with its SHA256 and
  source receipt recorded. **The canary retains `xyz.block.buzz.app` and may
  replace the normal installation and share its profile.** The workflow changes
  version/updater settings, not application identity. Use a separate Windows OS
  test account for an isolated profile, or the existing account with a newly
  added fixture community and its existing local data preserved. Installation
  and return to the normal build must be coordinated with the human.
  If using an existing identity, only its **public key** is needed by the fixture.
  Never export/paste its private key. A disposable Windows identity is also valid.
- A disposable Ubuntu 24.04 WSL distro (compatible with the GitHub Ubuntu debug
  binaries), or a disposable local Linux VM reachable only over local loopback.
  Windows `localhost` must reach WSL's loopback listener; verify that first.
  If the machine's WSL networking does not support this route, stop and record
  the prerequisite rather than exposing a public listener/tunnel.
- Local Postgres/Redis with newly created synthetic databases, no reused volumes,
  no production credentials. Apply the pinned candidate's schema only to that
  fresh database. The existing Linux CI rehearsal uses fresh service containers
  and migrations; `scripts/3720-registration-proof.sh` uses desired-state schema
  inside a no-egress namespace. Their source histories are different from a
  production migration compatibility check; #3751 owns the latter.
- The matching `buzz-relay`, `buzz-hostd` and source checkout for
  `scripts/paired-hosts/loopback_tls.py`. Linux CI artifacts are explicitly debug
  rehearsal artifacts, not production packages. Verify their `SHA256SUMS`.
- A fresh synthetic relay signing key held only in the fixture process/file
  (0600). Set `RELAY_OWNER_PUBKEY` to the public key selected in Windows. This
  bootstraps independent fixture membership; no agent credential or NIP-OA
  impersonation is involved.

The next section is for the conductor/WSL setup agent, not a request for Bryce
to reconstruct a fixture manually. Use [the bounded WSL handoff](wsl-fixture-handoff.md)
to prepare it, return a ready receipt, then give Bryce only the app observations.

## Loopback TLS and process setup

Use `https://localhost:39553` / `wss://localhost:39553` consistently. Generate a
one-day fixture CA and a server certificate with `subjectAltName=DNS:localhost`;
`registration-systemd-smoke.py` contains the exact OpenSSL commands. Keep both
private keys in the disposable Linux fixture. Copy **only the public CA
certificate** to Windows. Import it into the current user's root store and
record the returned thumbprint, for example in PowerShell:

```powershell
$fixtureCa = Import-Certificate -FilePath .\fixture-ca.crt -CertStoreLocation Cert:\CurrentUser\Root
$fixtureCa.Thumbprint
```

Windows WS uses native roots, while retaining certificate-chain and hostname
verification. Never use a certificate-validation bypass. The native executable
journey exercised the explicit private-CA host connection on Debian; Windows
native-root behavior remains part of this gate.

Start the fixture relay with synthetic `DATABASE_URL`, `REDIS_URL`,
`BUZZ_RELAY_PRIVATE_KEY`, `RELAY_OWNER_PUBKEY`, and these explicit settings:

```text
RELAY_URL=wss://localhost:39553
BUZZ_BIND_ADDR=127.0.0.1:39552
BUZZ_HEALTH_PORT=39554
BUZZ_METRICS_PORT=39555
BUZZ_HOST_REGISTRY_V2=true
BUZZ_PAIRED_HOSTS_V1=false
BUZZ_REQUIRE_AUTH_TOKEN=true
BUZZ_REQUIRE_RELAY_MEMBERSHIP=true
BUZZ_ALLOW_NIP_OA_AUTH=false
BUZZ_AUTO_MIGRATE=false
```

Set the disposable git/repository/cache directories too, as in the existing
rehearsal. Apply the fresh database schema before launching with auto-migrate
false. Wrap it with the existing loopback-only TLS helper:

```bash
python3 scripts/paired-hosts/loopback_tls.py --cert server.crt --key server.key \
  --https-port 39553 --relay-port 39552
```

Confirm NIP-11 over trusted TLS advertises `host_registry.versions=[2]`; record
its **public** relay key. It must match the key passed to hostd. Create a dedicated
unprivileged fixture account and an empty canonical 0700 state directory owned
by it. This is fixture setup, not adoption of an existing agent service. Under
that account, run the matching binary (all commands use the same absolute state
path):

```bash
buzz-hostd --state-dir /absolute/private/state registration init
buzz-hostd --state-dir /absolute/private/state registration run \
  --community https://localhost:39553 --relay-key PUBLIC_RELAY_KEY --ca-cert /absolute/fixture-ca.crt
```

In a second private terminal under the same account:

```bash
buzz-hostd --state-dir /absolute/private/state registration offer --community https://localhost:39553
```

The invitation requires a TTY. Paste it directly into Windows; do not capture it
in screenshots, shared logs, issue comments or this receipt. The hosted systemd
job independently tests the packaged unit's restricted root filesystem, service
identity and controller lock. A manually launched WSL process does not replace
that service-isolation result.

## Hands-on observations to return

1. Add the fixture community in Windows and select the matching unlocked owner.
   Open Agents → My hosts. With an empty supported directory, **Add Host** is
   visible. Cancel an unsubmitted invitation: no host appears.
2. Submit an expired or wrong-community invitation. It fails visibly; the input
   clears and retry remains available. Generate a fresh invitation and submit:
   observe pending registration, then **Registered**. The host row has no Add
   agent, installed-runtime, online, or deployment-capability claim.
3. While a command is awaiting a response, switch identity/community (or lock
   the identity). The new panel must not show the old result; the old native
   operation must not gain signing authority for the new session. Return to the
   fixture and retry from its correct active owner.
4. Restart hostd with the same state directory and restart Windows. Registration
   survives. For local journal-loss recovery, **close the canary**, back up only
   its device-local `paired-hosts` directory. For the unchanged release identity
   this is `%LOCALAPPDATA%\xyz.block.buzz.app\paired-hosts`. **Do this only in the
   separate Windows OS test account, or after the conductor establishes that the
   journal contains exclusively this fixture.** Move only that journal aside,
   preserve identity/profile files, reopen the same owner/community and observe
   the same host key and generation. In a shared existing profile, keep this
   sub-gate pending instead of disturbing other communities' retry journals.
   Another already-authorized desktop is an alternative; never export a key
   into chat.
5. Disconnect the fixture relay: the panel reports unavailable and retains saved
   registration information; it must not offer provisioning. Restore the relay,
   retry, then revoke the registration. Observe revocation pending followed by
   **Revoked**. Restart hostd and Windows again; revoked must stay revoked.
6. Return source SHA, artifact SHA256s, Windows/WSL versions, the exact observed
   sequence and any failure text with private invitation/key material removed.
   Human confirmation is recorded by the conductor; agents never self-attest it.

## Cleanup

Stop only the recorded fixture relay, TLS helper and hostd processes. Delete the
fixture database/containers/volumes, synthetic keys and host state, and remove
its dedicated account/distro when no longer needed. Remove the test community
from the tested application profile. Remove only the recorded fixture CA thumbprint:

```powershell
Remove-Item "Cert:\CurrentUser\Root\RECORDED_FIXTURE_CA_THUMBPRINT"
```

Do not reset the Windows profile or delete its identity files. Record cleanup and any intentionally
retained canary artifacts. Task #3750 stays open until the native journey,
hosted service gate, artifact checksums, independent review and human acceptance
are all recorded.
