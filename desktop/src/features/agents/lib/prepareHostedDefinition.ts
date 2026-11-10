import {
  prepareHostedAgent,
  type HostedSetup,
} from "@/shared/api/hostedAgents";
import type {
  AcpRuntime,
  AgentPersona,
  CreatePersonaInput,
  RegisteredHostDestination,
} from "@/shared/api/types";
import { buildInstanceInputForDefinition } from "./instanceInputForDefinition";
import { resolveManagedAgentAvatarUrl } from "../ui/managedAgentAvatar";

/** One native token and retained definition for one explicitly opened form draft. */
export type HostedDraft = {
  operation: string;
  isCurrent: () => boolean;
  runtime: string;
  harness: AcpRuntime;
  definition?: AgentPersona;
  frozen?: string;
};
export type HostedSubmitResult = { hosted: HostedSetup };
export type AgentSubmitResult = boolean | HostedSubmitResult;

/** Called behind the shared submit latch, before any local availability gate. */
export async function prepareHostedDefinition(
  input: CreatePersonaInput,
  destination: RegisteredHostDestination,
  draft: HostedDraft,
  createDefinition: (input: CreatePersonaInput) => Promise<AgentPersona>,
): Promise<HostedSubmitResult> {
  const check = () => {
    if (!draft.isCurrent())
      throw new Error("Host setup selection or workspace changed.");
  };
  check();
  const canonical = { ...input, runtime: draft.harness.id };
  const frozen = JSON.stringify(canonical);
  if (draft.frozen && draft.frozen !== frozen)
    throw new Error(
      "This prepared draft is frozen. Continue its retained setup.",
    );
  draft.frozen = frozen;
  if (!draft.definition) {
    const avatarUrl = await resolveManagedAgentAvatarUrl(
      input.avatarUrl,
      undefined,
      draft.harness.avatarUrl,
    );
    check();
    draft.definition = await createDefinition({ ...canonical, avatarUrl });
  }
  check();
  const agentInput = await buildInstanceInputForDefinition(
    draft.definition,
    draft.harness,
    undefined,
    destination,
  );
  check();
  return {
    hosted: await prepareHostedAgent(
      agentInput,
      destination.scope,
      draft.operation,
      draft.runtime,
    ),
  };
}
