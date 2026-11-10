import { toast } from "sonner";

import { attachManagedAgentToChannel } from "./channelAgents";
import type { Channel, CreateManagedAgentResponse } from "@/shared/api/types";

type TargetChannel = Pick<Channel, "id" | "name">;

async function attach(
  created: Pick<CreateManagedAgentResponse, "agent" | "spawnError">,
  targetChannel: TargetChannel,
) {
  const attached = await attachManagedAgentToChannel(targetChannel.id, {
    agent: created.agent,
    role: "bot",
    ...(created.agent.backend.type === "registered_host"
      ? {
          scope: {
            expectedRelayUrl: created.agent.backend.scope.community
              .replace(/^https:/, "wss:")
              .replace(/^http:/, "ws:"),
            expectedSignerPubkey: created.agent.backend.scope.owner,
          },
        }
      : {}),
    ensureRunning: created.agent.backend.type !== "registered_host",
  });
  created.agent = attached.agent;
}

function showAttachmentFailure(
  created: Pick<CreateManagedAgentResponse, "agent" | "spawnError">,
  targetChannel: TargetChannel,
  cause: unknown,
  toastId?: string | number,
) {
  const error = cause instanceof Error ? cause.message : "Failed to add agent.";
  const id = toast.warning("Agent created", {
    description: `${created.agent.name} couldn’t be added to #${targetChannel.name}. ${error}`,
    id: toastId,
    action: {
      label: "Try again",
      onClick: (event) => {
        event.preventDefault();
        toast.loading("Agent created", {
          description: `Adding ${created.agent.name} to #${targetChannel.name}…`,
          id,
        });
        void attach(created, targetChannel).then(
          () => {
            toast.success("Agent created", {
              description: `Added ${created.agent.name} to #${targetChannel.name}`,
              id,
            });
          },
          (retryCause: unknown) => {
            showAttachmentFailure(created, targetChannel, retryCause, id);
          },
        );
      },
    },
  });
}

/** Keeps creation successful when its optional channel attachment fails. */
export function useCreatedAgentChannelAttachment() {
  async function presentCreatedAgent(
    created: Pick<CreateManagedAgentResponse, "agent" | "spawnError">,
    targetChannel?: TargetChannel | null,
  ) {
    if (created.spawnError || !targetChannel) {
      toast.success("Agent created");
      return;
    }

    try {
      await attach(created, targetChannel);
      toast.success("Agent created");
    } catch (cause) {
      showAttachmentFailure(created, targetChannel, cause);
    }
  }

  return { presentCreatedAgent };
}
