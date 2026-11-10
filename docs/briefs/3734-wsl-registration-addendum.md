# #3734 — registration compatibility addendum only

Prepared after the user-supplied WSL receipt on November 10, 2026. This is a
follow-up for the same WSL session, not permission to deploy. Copy below the divider.

---

Continue #3734 preparation using Astra/high. Your completed relay build,
migration/catalog, signed-chat and exact-old rollback evidence remains credited.
Close only the missing registration-protocol compatibility check. No product
code changes, new framework, full CI, new canary, host installation or live work.
No merge or push authority. Keep the existing source and candidate image pinned.
No netcup access, existing agents, provider credentials or production identities.
Taskctl is local to netcup; report back rather than attempting to update it.

First verify your retained receipt/artifacts under
`/tmp/buzz-hosted-prep-929c-w39MsZNb`. The supplied sanitized receipt SHA256 is
`5900307f2796b37a72288f402e19b601d3cce9e9216157ad5d451532167f4ff0`.
Preserve it and append a separate addendum. Source is
`929c2cd890fbc2c0993549c3cbc64a2714232832`, tree
`1f1d902c2a80e528aaf639449fc1a114ea1129ad`.
Candidate amd64 image is
`sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f`;
exact old image is
`sha256:1da22b2856a998de760a12b0767b30f0daf30e65b580ff193054fa17a4c85044`.
Do not rebuild or substitute either relay image.

The netcup read-only Astra/high source inspection found existing external clients:

- `crates/buzz-paired-owner/examples/registration.rs`: owner `init`, `pair`, and
  authenticated `hosts` recovery against the supplied HTTPS origin/WSS relay.
  `BUZZ_REGISTRATION_CA` supplies a private fixture CA.
- `crates/buzz-hostd/src/registration_cli.rs`: host initialization/invitation and
  `registration run --community ORIGIN --relay-key PUBKEY --ca-cert CA --once`.
  Read its exact CLI/state arguments before invoking it.
- `scripts/paired-hosts/registration-systemd-smoke.py`: existing bounded PTY
  capture for invitation output. Reuse only that helper if necessary; do not run
  the script, install services, or start its separate relay implementation.

The runtime image does not contain these clients. Inspect the retained Docker
builder/cache. If available, build only these existing binaries in its `/build`
environment, at the exact source above:

```sh
cargo build --offline --locked --release -p buzz-hostd --bin buzz-hostd
cargo build --offline --locked --release -p buzz-paired-owner --example registration
```

Outputs are `target/release/buzz-hostd` and
`target/release/examples/registration`. Record hashes and source provenance.
Keep prior 6 GiB/2 CPU limits. A retained runnable builder is not assumed: if only
BuildKit cache remains, materialize the existing Dockerfile builder target using
the original context, arguments and pinned bases only if available through that
cache. Do not silently replace bases, install host Hermit/toolchains, download new
dependencies or launch a full rebuild. Return the concrete unavailable prerequisite
if this narrow build cannot run. Building a relay test executable alone is not
enough: the named registration tests instantiate a current-source relay internally
and would not exercise the pinned external images.

Use one new isolated synthetic fixture, because the earlier fixture was cleaned.
Reuse your existing PG/Redis/TLS/egress-denial setup and membership admission
mechanism, with no published ports or production secrets/endpoints. Keep the same
new database, synthetic keys and host journal across this addendum's transitions.
Do not SQL-seed the host registration; use the actual protocol:

1. On exact old/schema81, initialize a synthetic owner and host, admit only the
   owner as an ordinary fixture member, obtain the host invitation privately,
   publish owner `pair`, reconcile host with `--once`, and recover owner `hosts`.
   Require paired status and the expected owner/host/generation.
2. Apply migration82 through the existing candidate admin migrator. Check real
   registration reconciliation and authenticated owner recovery on exact old/82,
   candidate deployment-off, candidate deployment-on, candidate off again, then
   exact-old rollback. Registration remains enabled throughout. Deployment-on
   tests compatibility only: never invoke the deployment controller or create
   an agent. Use bounded reconciliation retries only where the protocol needs
   them; record failures rather than changing the product to make the test pass.
3. Ensure the recovery observation comes from authenticated relay reads rather
   than only a cached owner journal. Use the existing client's recovery path;
   if it caches the result, use a fresh disposable owner journal with the SAME
   synthetic signer using supported tooling. Do not involve a Windows profile.
4. Record the actual protocol outcomes and preserved binding identity at each
   transition. Do not repeat the full chat/catalog suite already credited or
   expand into revocation/lifecycle/provider testing. This is a separate
   registration-only fixture, not the deleted database from the original receipt.

Keep invitations/private keys and raw journals in private fixture storage. Return
only sanitized outcomes. Clean up only disposable resources from this addendum,
retaining evidence and the existing candidate/operator-plan artifacts. Have a
separate Astra/high reviewer assess this addendum and its exact-image/client
provenance; the prior accepted work is input, not another broad audit.

Return PASS or the precise remaining gap, client hashes/source, image IDs,
per-stage handshake/reconciliation/authenticated-read outcomes, cleanup evidence,
and reviewer disposition. No live SSH is required for this local gap. Production
staging, image loads, migrations, restarts, flags/config changes and signed live
events remain forbidden. Stop after the addendum; live approval is still separate.
