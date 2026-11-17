import { captureRelayRemovals } from "@/features/agents/managedAgentRelayCleanup";
import { openDm, sendChannelMessage } from "@/shared/api/tauri";
import { hostBelongsToSession } from "./hostDestination";
import type { Session } from "./hostDirectory";
import type {
  Channel,
  ManagedAgent,
  PresenceStatus,
  RelayAgent,
} from "@/shared/api/types";
import type { AgentAvailabilityReader } from "./useAgentAvailability";
import { normalizePubkey } from "@/shared/lib/pubkey";

type DeleteManagedAgentInput = {
  pubkey: string;
  forceRemoteDelete?: boolean;
};

type StartManagedAgent = (pubkey: string) => Promise<unknown>;
type StopManagedAgent = (pubkey: string) => Promise<unknown>;
type DeleteManagedAgent = (input: DeleteManagedAgentInput) => Promise<unknown>;

type ManagedAgentChannelContext = {
  channels: readonly Channel[];
  preferredChannelId?: string | null;
  relayAgents: readonly RelayAgent[];
};

type ManagedAgentActionContext = ManagedAgentChannelContext & {
  getAvailability: AgentAvailabilityReader;
};

export type ManagedAgentActionResult = {
  cancelled?: boolean;
  noticeMessage?: string;
};

/** Lifecycle action routing only; deployed is a retained receipt, not presence. */
export function isManagedAgentActive(agent: Pick<ManagedAgent, "status">) {
  return agent.status === "running" || agent.status === "deployed";
}

export function getManagedAgentPrimaryAction(
  agent: ManagedAgent,
  availability: PresenceStatus | undefined,
  session?: Session,
) {
  if (agent.backend.type === "registered_host") {
    if (!session || !hostBelongsToSession(agent.backend, session))
      return undefined;
    const stop = availability === "online" || availability === "away";
    return { stop, label: stop ? "Stop" : "Start agent" };
  }

  const stop = isManagedAgentActive(agent);
  if (agent.backend.type === "provider") {
    return { stop, label: stop ? "Shutdown" : "Deploy" };
  }
  return { stop, label: stop ? "Stop" : "Start agent" };
}

export type ManagedAgentPrimaryAction = NonNullable<
  ReturnType<typeof getManagedAgentPrimaryAction>
>;

export function resolveManagedAgentChannelId(
  agent: Pick<ManagedAgent, "pubkey">,
  context: ManagedAgentChannelContext,
) {
  if (context.preferredChannelId) {
    return context.preferredChannelId;
  }

  const relayAgent = context.relayAgents.find(
    (candidate) =>
      normalizePubkey(candidate.pubkey) === normalizePubkey(agent.pubkey),
  );

  if (relayAgent?.channelIds?.length) {
    return relayAgent.channelIds[0];
  }

  const channelName = relayAgent?.channels?.[0];
  if (!channelName) {
    return null;
  }

  const matches = context.channels.filter(
    (channel) => channel.name === channelName,
  );
  return matches.length === 1 ? matches[0].id : null;
}

export async function startManagedAgentWithRules({
  agent,
  startManagedAgent,
}: {
  agent: ManagedAgent;
  startManagedAgent: StartManagedAgent;
}) {
  // Relay-mesh agents are no longer blocked here: the backend start preflight
  // (ensure_relay_mesh_for_record) re-resolves a live serve target and dials
  // it, failing with an actionable error when no peer serves the model.
  await startManagedAgent(agent.pubkey);
}

export async function respawnManagedAgentWithRules({
  agent,
  relayUrl,
  startManagedAgent,
  stopManagedAgent,
  onStopped,
}: {
  agent: ManagedAgent;
  /** The active community's relay: the workspace pair the native stop and
   * start target. A removal of it during the stop cancels the start. */
  relayUrl: string | undefined;
  startManagedAgent: StartManagedAgent;
  stopManagedAgent: StopManagedAgent;
  /** Called after a successful stop and before start begins — use this to
   * clear stale working badges at the right boundary. */
  onStopped?: () => void;
}) {
  if (agent.backend.type === "local" && isManagedAgentActive(agent)) {
    const assertRelayNotRemoved = relayUrl
      ? captureRelayRemovals(relayUrl)
      : () => {};
    await stopManagedAgent(agent.pubkey);
    onStopped?.();
    assertRelayNotRemoved();
  }

  await startManagedAgent(agent.pubkey);
}

export async function stopManagedAgentWithRules({
  agent,
  channels,
  preferredChannelId,
  relayAgents,
  session,
  stopManagedAgent,
}: {
  agent: ManagedAgent;
  stopManagedAgent: StopManagedAgent;
  session?: Session;
} & ManagedAgentChannelContext): Promise<ManagedAgentActionResult> {
  const hosted =
    agent.backend.type === "registered_host" ? agent.backend : undefined;
  if (hosted || agent.backend.type === "provider") {
    if (hosted && (!session || !hostBelongsToSession(hosted, session))) {
      throw new Error(
        "Cannot stop: select the registered host’s community and owner identity.",
      );
    }
    const expectedRelayUrl = hosted ? session?.expectedRelay : undefined;
    const expectedSignerPubkey = hosted?.scope.owner;
    const pubkeys = [agent.pubkey];
    const channelId = hosted
      ? (await openDm({ pubkeys, expectedRelayUrl, expectedSignerPubkey })).id
      : resolveManagedAgentChannelId(agent, {
          channels,
          preferredChannelId,
          relayAgents,
        });
    if (!channelId) {
      throw new Error("Cannot stop: agent is not in any channel");
    }

    await sendChannelMessage(
      channelId,
      "!shutdown",
      undefined,
      undefined,
      pubkeys,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      expectedRelayUrl,
      expectedSignerPubkey,
    );
    return {
      noticeMessage:
        "Shutdown requested. This does not confirm the agent has stopped.",
    };
  }

  await stopManagedAgent(agent.pubkey);
  return {};
}

export async function deleteManagedAgentWithRules({
  agent,
  channels,
  deleteManagedAgent,
  preferredChannelId,
  getAvailability,
  relayAgents,
  skipRemoteDeleteConfirm = false,
}: {
  agent: ManagedAgent;
  deleteManagedAgent: DeleteManagedAgent;
  skipRemoteDeleteConfirm?: boolean;
} & ManagedAgentActionContext): Promise<ManagedAgentActionResult> {
  if (agent.backend.type === "provider" && agent.backendAgentId) {
    const availability = getAvailability(agent.pubkey);
    const channelId = resolveManagedAgentChannelId(agent, {
      channels,
      preferredChannelId,
      relayAgents,
    });

    if (channelId) {
      // Only established Offline preserves the intentional no-request path.
      // Unknown is not evidence that shutdown can safely be skipped.
      if (availability !== "offline") {
        await sendChannelMessage(channelId, "!shutdown", undefined, undefined, [
          agent.pubkey,
        ]);

        if (!skipRemoteDeleteConfirm) {
          const confirmed = window.confirm(
            (availability === undefined
              ? "This agent’s availability is unknown. "
              : "") +
              "Shutdown requested, but the agent may still be running. " +
              "Deleting now removes the local record — the remote deployment " +
              "will be orphaned if shutdown hasn't completed. Continue?",
          );
          if (!confirmed) {
            return { cancelled: true };
          }
        }
      } else {
        if (!skipRemoteDeleteConfirm) {
          const confirmed = window.confirm(
            "This agent is offline but the remote deployment may still exist. " +
              "Deleting removes the local management record. Continue?",
          );
          if (!confirmed) {
            return { cancelled: true };
          }
        }
      }
    } else {
      if (!skipRemoteDeleteConfirm) {
        const confirmed = window.confirm(
          "This agent is deployed but not in any channel. " +
            "Deleting removes the local management record; the remote deployment may still be running. Continue?",
        );
        if (!confirmed) {
          return { cancelled: true };
        }
      }
    }
  }

  const isDeployedRemote =
    agent.backend.type === "provider" && agent.backendAgentId;
  await deleteManagedAgent({
    pubkey: agent.pubkey,
    forceRemoteDelete: isDeployedRemote ? true : undefined,
  });

  return {};
}
