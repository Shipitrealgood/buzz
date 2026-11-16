# #2722 — approved Hetzner discovery rollout

Bryce explicitly approved updating Hetzner and netcup on 2026-11-16, using
Astra/high, preserving Claude/Grok/Scout and preparing one Windows test. This
supersedes the read-only stopping point, not the pinned artifact or scope.
Netcup activation remains conductor-owned and follows the successful relay receipt.

---

Use Astra/high. Bryce approves the prepared Hetzner primary-relay update now.
Use your existing verified staging and proposed commands at
`claude/findings/relay-d7714be8-transfer-cq5CTn/proposed-commands.txt` and
`verified-artifact.json`. No additional approval is needed for the exact update
and emergency recovery described below. No new code, rebuild, migration,
test community, publication or broad compatibility campaign.

Only Hetzner's primary relay is yours. Do not access netcup or change/restart
Claude, Grok, Scout, Caddy, pair-relay or dependencies. Do not create agents,
change provider logins, generate signed test events, or modify taskctl. The
conductor updates netcup only after you return a healthy relay receipt.

1. Recheck the staged archive hash and narrow live drift checks from your accepted
   readiness receipt. Keep the verified SSH route and strict host-key checks.
   Expected hostname ubuntu-4gb-fsn1-3; relay identity
   `9fd495a8df792128e1396c3766b12228076cad0ea7b791c3d5fbc446f74d62c1`.
   Stop on unexpected identity, image, configuration, access or trust drift;
   do not alter firewall/SSH/access. Preserve the existing recovery assets and
   exact current Compose/configuration for rollback. No new database checkpoint,
   database restore or traffic shutdown beyond the primary-container replacement.

2. Stream/load the already-verified archive into Hetzner Docker using your
   prepared transfer command. Archive SHA256:
   `3705c0a5d79288ce72bb3d70419e00d2180d3eb7fdf72f7879358ef6491901c4`.
   Source `d7714be862f4514822c303b124a347f5e07dc8c5`; linux/amd64.
   Match the loaded image to the exact OCI-index/platform/config mapping you
   verified, not just its tag. Recorded index:
   `sha256:3e7da8876903702f11898dc467d4603a39eae57249e13e335cacbcb175cbced3`.
   Stop on mismatch before recreating anything. Retain the old image.

3. Stage the prepared image-only override. Using your measured seven-file Compose
   order plus that override, clean-environment comparison must permit only the
   primary image change. Keep registryV2/deploymentV1/auth-token/membership true,
   auto-migrate/paired-hostsV1 false, and NIP-FI unchanged (unset/default Off).
   Keep all storage, keys, listeners, networks and other services unchanged.
   Candidate migration files equal old929c; no migration command is permitted.
   Apply only `up -d --no-deps --no-build --pull never relay` with the exact
   prepared prefix. No lifecycle wrapper, pull, build, down or dependency restart.

4. Verify the running image mapping/source, healthy HTTPS/TLS, unchanged relay
   identity, root and /pair WebSocket behavior, NIP42 challenge, unsigned-query
   denial and existing registry/deployment capability advertisement. Compare all
   six nonprimary service IDs/start times/configuration identities to baseline.
   Capture the actual interruption and bounded diagnostics; report native human
   chat/discovery as not run. Never present capability success as a created agent.

5. Emergency recovery is authorized if integrity/auth/identity checks fail, or
   ordinary relay health remains unavailable for90seconds. Use your prepared
   exact old image `sha256:5bc530077c757a8b058c08ef6db9e17eba8f0551118d88d17388db972fceaa0f`
   with BOTH `BUZZ_HOST_REGISTRY_V2=false` and `BUZZ_HOST_DEPLOYMENT_V1=false`,
   preserving auto-migrate=false, auth and every other setting. Use the same
   checked primary-only Compose procedure, preserve the current database/history,
   and verify ordinary health plus absent host-management capabilities. Never
   re-enable management under old929c. This fallback is source-reviewed, including
   HTTP43103 rejection in required_scope_for_kind; post-candidate runtime recovery
   remains untested unless actually exercised. Report any recovery immediately;
   the conductor will keep netcup activation held. Do not loop through updates.

6. Obtain one bounded independent Astra/high review of the actual change/receipt,
   reusing the completed source and readiness evidence. No replay of the earlier
   synthetic test matrix or new implementation. Preserve private logs/configs;
   return a sanitized receipt with image/source/schema disposition, flags,
   identity, health/interruption, unchanged nonprimary services, recovery location
   and any remaining findings. Stop at RELAY READY (or the precise failure).

Windows Add Agent/setup remains closed until the conductor confirms both backends
are ready. No installer or provider operation is part of this WSL rollout.
