# #3750 execution ledger

Executor: `/root/windows_registration_astra_exec`, native codecodex dispatch.
Requested gpt-6-astra/high; runtime identity not independently exposed.
Workspace: `/home/bryce/services/buzz-wt-3720-host-registration`.
Branch: `proof/3720-host-registration-events`.
Starting clean revision: `621c777b886dad8aed7eeee4b428d57c153829e5`.
Code commits: `8404ea2c3070622eeffc7341a24ccb4b89f08d4a`,
`0026180c1ee00ca3f3eb583d30ca14302a415072` (signed off).

Status: implementation ready for independent review; **#3750 acceptance open**.
Final local measured gates and limits: [run index](run-index.json).
Physical line changes/source hashes/local artifact checksums: [source audit](source-audit.json).
No unrelated changes, board writes, pushes, merges, live services or messages.
Fixture resources and source/test processes stopped: [cleanup](cleanup.json).

Next: conductor dispatches independent Astra/high reviewer in this same pinned
worktree. After accepted review, conductor owns existing hosted systemd and
Windows canary workflow actions. Native Windows journey/human acceptance remain
pending; [Windows steps](windows-journey.md) and [WSL-agent setup](wsl-fixture-handoff.md)
are prepared. Canary uses the ordinary application identifier, not a new profile.
