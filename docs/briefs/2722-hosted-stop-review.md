# Review Windows hosted Stop for #2722

Review the change from `f1c6325aa84f7068912ea9e56c97b4292330a7ee` on `proof/3720-host-registration-events`. The requested behavior is step 3 of the [completion plan](/home/bryce/services/buzz-agents/docs/specs/2722-hosted-agent-completion.md).

- Follow the primary action from the selected agent's profile and the channel-member menu into `managedAgentControlActions.ts`. Both use the rendered decision for the click. A registered host must match the active owner and community. Online and Away offer Stop. Offline and unknown use the existing guarded Start.
- For hosted Stop, the profile resolves the selected key in the relay directory. The member menu uses the selected member's channel. The existing message carries the literal `!shutdown`, one agent mention, the captured WebSocket relay URL and the host scope's owner key. The native send validates the relay and signer before publication. A successful send reports “Shutdown requested.” Process exit requires a separate observation.
- The member badge uses the same scoped presence. Its saved record can still say stopped while the remote agent is away. Bulk target selection, host Start admission, provider and local lifecycle behavior follow their existing paths.

## Verification

All 51 cases in the three focused JavaScript files passed (8 control actions, 11 availability, 32 deletion). The TypeScript and Vite E2E build passed. Biome passed for all 11 touched TypeScript, TSX and JavaScript files. The new synthetic browser cases are in `hosted-agent-setup.spec.ts` and `agent-availability.spec.ts`. All 10 matched Chromium cases passed in 1.1 minutes. They cover hosted profile and member-menu Stop, unavailable channel, wrong owner/community, offline Edit and Start, provider shutdown and local per-community controls. Earlier mock runs exposed a nonexistent fixture channel and a mismatch between the HTTPS host origin and the expected WebSocket relay URL. The final fixture uses the existing agents stream, and the send uses the captured session relay after checking the stored origin.

The independent source gate and the installed Windows run against one disposable netcup agent are pending. For the latter, observe the unit exit, restart the same identity and verify its history and reply. The existing untracked hosted-edit audit and management design belong to the conductor.
