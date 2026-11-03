# Linux paired host

Run the control plane under a dedicated Linux account with an empty HOME. This account owns one host key and journal, and must be separate from desktop, relay-admin, agent-runtime and operator accounts. Run these commands in that account on a private console:

```sh
install -d -m 0700 "$PWD/host-state"
buzz-hostd --state-dir "$PWD/host-state" init
buzz-hostd --state-dir "$PWD/host-state" offer --community https://team.example
buzz-hostd --state-dir "$PWD/host-state" status
buzz-hostd --state-dir "$PWD/host-state" serve --catalog "$PWD/catalog.json"
```

`offer` prints a ten-minute single-use JSON invitation to an interactive terminal. Import it into the selected community on the owner device. The journal retains only the proof digest until consumption, five distinct invalid attempts, or expiration. A new offer replaces any outstanding invitation. Successful pairing persists one owner, community, generation and the exact signed acknowledgement. Service restarts retry pending HTTP delivery. Shut off console/session recording before issuing an invitation.

To revoke while offline, copy the exact generation from `status` and run `buzz-hostd --state-dir "$PWD/host-state" unpair --generation "$GENERATION"`. Local revocation takes effect on commit. The service uploads the same signed tombstone on reconnect. Keep the journal for historical delivery and reconciliation.

Treat the SQLite database as a credential store. Grant-store loss or a potentially stale backup requires a fresh host key and enrollment in a new empty state directory. Starting a restored copy can resurrect authority, so keep old journals offline during recovery.

## Catalog and service account

Catalog IDs use `plugin/type`. The catalog is a local operator artifact. Each entry pins the absolute, canonical path and SHA-256 of a conversational `buzz-acp` harness and one adapter. Both executables and their ancestors must be root-owned and unwritable by group/others. A catalog revision covers the complete file, including limits, fixed adapter args and the credential profile name. The OS trust store itself is managed separately. The key in `relay_key` is the community relay signing public key for authoritative roster and presence snapshots. Obtain it from the operator.

```json
{
  "community": "https://team.example",
  "label": "Linux pilot",
  "capacity": 2,
  "relay_key": "<64 lowercase hex characters>",
  "runtimes": {
    "local/goose": {
      "harness": {"path":"/usr/local/libexec/buzz-acp","sha256":"<digest>"},
      "adapter": {"path":"/usr/local/libexec/goose","sha256":"<digest>"},
      "args":["acp"],
      "credential_profile":"none"
    }
  }
}
```

A named provider profile selects `/etc/buzz-hostd/providers/<name>.json`. Install that file root-owned, mode 0600. Its root-owned directory must allow the host service to stat the file at commit. Only the installed root helper reads its contents. It contains only the explicitly supported provider API key variables. The profile named `none` needs no provider secret.

The production service expects the control journal at `/var/lib/buzz-hostd/host.sqlite` and catalog at `/etc/buzz-hostd/catalog.json`. Install the three binaries `buzz-hostd`, `buzz-host-runtime` and `buzz-host-activate` root-owned under `/usr/local/libexec`. Install the four units from `deploy/` into `/etc/systemd/system`. Create the system user and group `buzz-hostd` with a disabled login. Give it only its own mode-0700 state directory. Initialize its journal as that user and issue the console invitation. The OS operator explicitly runs `systemctl daemon-reload` and `systemctl enable --now buzz-hostd-activate.socket buzz-hostd.service`. Check the catalog executables, their complete dependencies and the service confinement on the intended OS before enabling this.

The host verifies committed credentials and checks the exact public profile, directory, registration and every requested channel's relay-signed roster. It stages the candidate key, OA, exact scope, runtime selector, frozen catalog revision and requested policies under its own UID. It sends only fixed local actions and a derived 64-hex instance ID to the root socket. The root helper checks SO_PEERCRED against the dedicated service account, serializes root reservations against its independent capacity ceiling, and refuses foreign templates or overrides. It resolves runtime and provider authority independently from `/etc/buzz-hostd/catalog.json`. Worker credentials are immutable in a root-only store. A root reservation creates no process. The daemon journals the start intent before asking the helper to enable the fixed instance.

The DynamicUser launcher reopens the root catalog at every boot and starts the pinned harness with an empty environment and its own provider profile. The paired unit uses `Type=notify` and a 75-second startup timeout. The main harness sends readiness only after eager ACP initialization and relay EOSE for membership and all requested channel subscriptions on the current connection. The journal separately requires confirmed writer-side heads, per-channel roster and presence before reporting ready. A failed start or detected post-start conflict quiesces only the owned instance; recovery repeats pending cleanup after a lost reply. An initial missing membership remains awaiting the owner's signed channel addition. Only the submitting owner device sees a refusal of its 9000 publication. An empty-condition OA is a portable credential for that individual agent after delivery.

## Isolated rehearsal

Build with `cargo build --locked -p buzz-hostd`. The two binaries for unit activation and worker launch use Linux/systemd. Local protocol tests use ephemeral keys and real SQLite, without any real service-manager calls. A new journal in another location can exchange protocol messages and verify public relay records, and reports awaiting system bootstrap before attempting actual unit activation.

For a disposable GitHub-hosted Ubuntu VM, build the three host binaries, the `buzz` CLI and the `buzz-acp` harness into a single bin directory. A test relay using an ephemeral CA needs consumers built with native TLS roots; enable the `tokio-tungstenite/rustls-tls-native-roots` feature for the staging ACP/CLI build. Run:

```sh
sudo --preserve-env=GITHUB_ACTIONS,RUNNER_ENVIRONMENT \
  crates/buzz-hostd/deploy/stage-on-actions.sh "$BIN_DIR" "$STAGING_CA" "$RELAY_PUBKEY"
sudo -u buzz-hostd /usr/local/libexec/buzz-hostd --state-dir /var/lib/buzz-hostd status
```

The bootstrap installs its ephemeral CA on that disposable VM, a digest-pinned `local/fixture` ACP adapter and the isolated service units. If scripting the private console enrollment for E2E, use a nonlogged pseudo-terminal redirected into a 0600 file under the private test directory. Disable shell tracing. After a successful create, check the exact signed result, each channel roster, `systemctl is-active` and `systemctl is-enabled` for the opaque instance, and an eligible and denied message from independent clients. Studio/UI tests, OS sandbox tests and production rollout are separate gates.
