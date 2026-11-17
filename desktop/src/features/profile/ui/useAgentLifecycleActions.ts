import * as React from "react";
import { toast } from "sonner";

import {
  getManagedAgentPrimaryAction,
  isManagedAgentActive,
  respawnManagedAgentWithRules,
  startManagedAgentWithRules,
  stopManagedAgentWithRules,
} from "@/features/agents/lib/managedAgentControlActions";
import { agentPresenceStartBlockReason } from "@/features/agents/lib/useAgentAvailability";
import { clearActiveTurnsForAgentOnStop } from "@/features/agents/managedAgentRuntimeHooks";
import { isRelayRemovedError } from "@/features/agents/managedAgentRelayCleanup";
import { useCommunities } from "@/features/communities/useCommunities";
import type {
  Channel,
  ManagedAgent,
  RelayAgent,
  PresenceStatus,
} from "@/shared/api/types";

export function useAgentLifecycleActions({
  availability,
  currentPubkey,
  channels,
  managedAgent,
  relayAgents,
  startManagedAgent,
  stopManagedAgent,
}: {
  availability: PresenceStatus | undefined;
  currentPubkey?: string;
  channels: readonly Channel[] | undefined;
  managedAgent: ManagedAgent | undefined;
  relayAgents: readonly RelayAgent[] | undefined;
  startManagedAgent: (pubkey: string) => Promise<unknown>;
  stopManagedAgent: (pubkey: string) => Promise<unknown>;
}) {
  const relayUrl = useCommunities().activeCommunity?.relayUrl;
  const session = React.useMemo(
    () =>
      relayUrl && currentPubkey
        ? { expectedRelay: relayUrl, expectedOwner: currentPubkey }
        : undefined,
    [relayUrl, currentPubkey],
  );
  const primaryAction =
    managedAgent &&
    getManagedAgentPrimaryAction(managedAgent, availability, session);
  const handleAgentPrimaryAction = React.useCallback(async () => {
    if (!managedAgent || !primaryAction) return;

    try {
      if (primaryAction.stop) {
        const result = await stopManagedAgentWithRules({
          agent: managedAgent,
          session,
          channels: channels ?? [],
          relayAgents: relayAgents ?? [],
          stopManagedAgent,
        });
        if (managedAgent.backend.type === "local") {
          clearActiveTurnsForAgentOnStop(managedAgent.pubkey);
        }
        toast.success(result.noticeMessage ?? `Stopped ${managedAgent.name}.`);
        return;
      }

      const blockReason = agentPresenceStartBlockReason(false, availability);
      if (blockReason) throw new Error(blockReason);
      await startManagedAgentWithRules({
        agent: managedAgent,
        startManagedAgent,
      });
      toast.success(
        managedAgent.backend.type === "registered_host"
          ? `Start requested for ${managedAgent.name}.`
          : managedAgent.backend.type === "provider"
            ? `Deploying ${managedAgent.name}.`
            : `Started ${managedAgent.name}.`,
      );
    } catch (error) {
      toast.error(
        error instanceof Error ? error.message : "Agent action failed.",
      );
    }
  }, [
    availability,
    channels,
    primaryAction,
    session,
    managedAgent,
    relayAgents,
    startManagedAgent,
    stopManagedAgent,
  ]);

  const handleAgentRestart = React.useCallback(async () => {
    if (!managedAgent) return;

    try {
      const blockReason = agentPresenceStartBlockReason(
        isManagedAgentActive(managedAgent),
        availability,
      );
      if (blockReason) throw new Error(blockReason);
      await respawnManagedAgentWithRules({
        agent: managedAgent,
        relayUrl,
        startManagedAgent,
        stopManagedAgent,
        onStopped: () => clearActiveTurnsForAgentOnStop(managedAgent.pubkey),
      });
      toast.success(`Restarted ${managedAgent.name}.`);
    } catch (error) {
      if (isRelayRemovedError(error)) return;
      toast.error(
        error instanceof Error ? error.message : "Agent restart failed.",
      );
    }
  }, [
    availability,
    managedAgent,
    relayUrl,
    startManagedAgent,
    stopManagedAgent,
  ]);

  return { primaryAction, handleAgentPrimaryAction, handleAgentRestart };
}
