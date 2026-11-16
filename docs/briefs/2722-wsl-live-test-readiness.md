# #2722 — existing-community discovery test: WSL readiness

Bryce chose to test the already-built implementation first and defer discovery
simplification. This supersedes the earlier WSL image-build request. No new build,
test community, product change or isolated compatibility campaign is needed.
Copy the text below into the existing WSL Codex session.

---

Use Astra/high. Prepare the existing Hetzner relay for the coordinated Windows
test.28/netcup hostd discovery test. You own read-only Hetzner inspection and local
WSL artifact staging plus the concrete primary-only update/recovery proposal.
Stop after returning that proposal; no live image load, configuration change,
restart, flag change, migration, signed event, agent creation or provider call is
authorized by this handoff. No source edits, commits, pushes, merges or taskctl.
The conductor handles netcup activation separately. Existing Claude/Grok/Scout,
Jeff/test1, pairing and community state must remain unchanged.

The goal is ordinary Add Agent → netcup → installed harness → returned model →
Create in Bryce's existing community. Source and actual-relay integration reviews
passed. Windows test.28 built with 125 passing native cases; matching hostd passed
compatibility with retained runtime/ACP. These are credited results, not requests
to repeat tests. Real provider conversation and native human acceptance remain.

First use your retained November relay rollout/flag-enable receipts and existing
SSH route, then measure current state. Historical expected values below are drift
checks, not fresh observations. Stop on access/trust/identity mismatch; no SSH,
firewall, account, key or network changes.

Verified historical route:
ssh -o BatchMode=yes -o ConnectTimeout=8 -o StrictHostKeyChecking=yes -o UpdateHostKeys=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_hetzner root@2.31.66.65

Expected hostname: ubuntu-4gb-fsn1-3.
Compose directory: /mnt/HC_Volume_107262239/buzz/deploy/compose.
Relay: wss://buzz.interleaven.com.
Public identity: 9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1.
Last accepted image: sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f.
Last accepted source: 929c2cd890fbc2c0993549c3cbc64a2714232832; schema82.
RegistryV2/deploymentV1/auth-token/membership true; auto-migrate/paired-hostsV1
false; NIP-FI unset with source default Off. Measure only these flags, image,
ordered Compose files, service IDs/start times and normal endpoint/capability
health. No full environment/configuration dumps or application-record reads.
Use a read-only database transaction for schema/ledger checks if needed.

The image is ALREADY BUILT. WSL-to-netcup SSH/Tailscale access is intentionally
absent; do not attempt it or create access. Bryce authorized GitHub distribution.
Download from the authenticated draft release in Shipitrealgood/buzz, tag
host-discovery-d7714be8-transfer. It is unpublished, so use gh authenticated as an
account with access to the draft, not an anonymous download URL. Do not print
tokens. Stop on unavailable GitHub access; never publish the draft to bypass it.

```bash
umask 077
buzz_stage=$(mktemp -d "$HOME/buzz-discovery-d7714be8.XXXXXX")
gh release download host-discovery-d7714be8-transfer \
  --repo Shipitrealgood/buzz --dir "$buzz_stage" \
  --pattern buzz-relay-d7714be8-linux-amd64.tar.gz \
  --pattern SHA256SUMS --pattern relay-artifact.json --pattern WSL-HANDOFF.md
cd "$buzz_stage"
sha256sum -c SHA256SUMS
printf '%s  %s\n' \
  3705c0a5d79288ce72bb3d70419e00d2180d3eb7fdf72f7879358ef6491901c4 \
  buzz-relay-d7714be8-linux-amd64.tar.gz | sha256sum -c -
```

Archive size98,378,289 bytes; relay-artifact.json contains source/platform and
archive/image pins. If narrow source inspection is needed, use an existing or
temporary local checkout of public Shipitrealgood/buzz at exact commitd771 below
and old929c. No build, source edit, merge or push. The conductor has separately
retained the exact source archive and full build provenance on netcup.

Cohort source d7714be862f4514822c303b124a347f5e07dc8c5; standard root Dockerfile
runtime target, linux/amd64. BuildKit recorded image/OCI-index identity:
sha256:3e7da8876903702f11898dc467d4603a39eae57249e13e335cacbcb175cbced3.
Do not confuse an OCI-index digest with a platform image/config ID on a different
Docker image store. Verify the archive hash and its platform/config mapping; record
the exact identity the target Docker will use rather than guessing. No rebuild or
registry publication. Local WSL image loading is optional for resolving that
mapping; do not start a relay/container or touch the live Docker image store.

Prepare a NONEXECUTED image-only primary-relay override using the complete measured
Compose file order and clean-environment configuration comparison. All flags,
auth, keys, volumes, networks/listeners and nonprimary services stay unchanged.
Candidate migrations/schema are source-identical to929c; no migration command.
Use primary-only up --no-deps --no-build --pull never relay, never the broad
lifecycle wrapper. Define ordinary health/auth/identity checks and preserve the
existing encrypted recovery assets; no new production checkpoint or restore now.

Recovery boundary to resolve narrowly in the proposal:
The new relay restricts models-query/models historical reads to active bindings.
Old929c read_registry messages/evidence lacks that filter; its reply path does not
reject unknown inner operations. Consequently old-image rollback after new model
traffic is NOT security-equivalent even though the database schema is unchanged.
Pausing hostd while retaining the new relay covers host-management failures only;
it does not cover failure of the relay itself. Inspect the pinned old source and
existing flags to determine whether an emergency old-image configuration with
both registry and deployment disabled can restore ordinary chat while denying
all host-management reads/admission. Propose it only if the source supports that
claim; state untested runtime aspects. Do not implement a patch, new gate framework,
delete history, restore a database, or silently accept weaker revocation. If this
cannot be established, report the concrete gap. This is a finite recovery-plan
check, not a request to redesign discovery or rerun the old six-stage matrix.

The hostd rollback separately requires candidate-owned pending models replies to
drain/expire before stopping and reverting the controller; preserve its current
journal. Netcup's conductor owns that procedure. Keep Windows setup closed during
the coordinated update; do not ask the user to test until both backends match.

Return a concise sanitized receipt: measured live state/drift, local artifact
hash/path/platform identity or transfer gap, exact proposed transfer/load and
primary-only update commands, the emergency recovery disposition and remaining
unknowns. Reuse your existing comparator/runbook where applicable. No local taskctl
access is required. Stop before any live mutation and return for the conductor's
coordinated approval checkpoint.
