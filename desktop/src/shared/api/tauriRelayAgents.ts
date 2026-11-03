import {
  fromRawRelayAgent,
  invokeTauri,
  type RawRelayAgent,
} from "@/shared/api/tauri";
import type { RelayAgent } from "@/shared/api/types";

export async function revalidateRelayAgents(
  pubkeys: string[],
  channelId: string | undefined,
  expectedRelayUrl: string,
  expectedSignerPubkey: string,
): Promise<RelayAgent[]> {
  const agents = await invokeTauri<RawRelayAgent[]>("revalidate_relay_agents", {
    pubkeys,
    channelId,
    expectedRelayUrl,
    expectedSignerPubkey,
  });
  return agents.map(fromRawRelayAgent);
}

export async function checkMemberAddPeople(
  pubkeys: string[],
  expectedRelayUrl: string,
  expectedSignerPubkey: string,
): Promise<string[]> {
  const people: string[] = [];
  for (let offset = 0; offset < pubkeys.length; offset += 50) {
    people.push(
      ...(await invokeTauri<string[]>("check_member_add_people", {
        pubkeys: pubkeys.slice(offset, offset + 50),
        expectedRelayUrl,
        expectedSignerPubkey,
      })),
    );
  }
  return people;
}
