import {
  fromRawRelayAgent,
  invokeTauri,
  type RawRelayAgent,
} from "@/shared/api/tauri";
import type { RelayAgent } from "@/shared/api/types";

export async function revalidateRelayAgents(
  pubkeys: string[],
  channelId?: string,
  expectedRelayUrl?: string,
  expectedSignerPubkey?: string,
): Promise<RelayAgent[]> {
  const agents = await invokeTauri<RawRelayAgent[]>("revalidate_relay_agents", {
    pubkeys,
    channelId,
    expectedRelayUrl,
    expectedSignerPubkey,
  });
  return agents.map(fromRawRelayAgent);
}
