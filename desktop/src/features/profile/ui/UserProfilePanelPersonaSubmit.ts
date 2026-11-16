import { toast } from "sonner";

import { personaManagedAgentUpdate } from "@/features/profile/ui/UserProfilePanelUtils";
import type {
  AcpRuntimeCatalogEntry,
  AgentPersona,
  CreateManagedAgentResponse,
  CreatePersonaInput,
  ManagedAgent,
  UpdateManagedAgentInput,
  UpdatePersonaInput,
} from "@/shared/api/types";

/** Exact host selection accompanying the ordinary definition Save. */
export type HostedEditSelection = {
  installedRuntime: string;
  runtime?: AcpRuntimeCatalogEntry | null;
};

type SubmitProfilePersonaDialogOptions = {
  hosted?: HostedEditSelection;
  createManagedAgentForPersona: (
    persona: AgentPersona,
  ) => Promise<CreateManagedAgentResponse>;
  createPersona: (input: CreatePersonaInput) => Promise<AgentPersona>;
  input: CreatePersonaInput | UpdatePersonaInput;
  managedAgent: ManagedAgent | undefined;
  onDone: () => void;
  previousPersona?: AgentPersona;
  runtimes?: readonly AcpRuntimeCatalogEntry[];
  updateManagedAgent: (
    input: UpdateManagedAgentInput,
  ) => Promise<{ agent: ManagedAgent; profileSyncError: string | null }>;
  updatePersona: (input: UpdatePersonaInput) => Promise<AgentPersona>;
};

type ValidateLinkedAgentRuntimeEditOptions = {
  input: UpdatePersonaInput;
  managedAgent: ManagedAgent | undefined;
  previousPersona?: AgentPersona;
  runtimes?: readonly AcpRuntimeCatalogEntry[];
};

function normalizeRuntimePreference(value: string | null | undefined): string {
  return value?.trim() ?? "";
}

export function validateLinkedAgentRuntimeEdit({
  input,
  managedAgent,
  previousPersona,
  runtimes,
}: ValidateLinkedAgentRuntimeEditOptions): string | null {
  if (!managedAgent || !previousPersona) {
    return null;
  }

  const previousRuntime = normalizeRuntimePreference(previousPersona.runtime);
  const nextRuntime = normalizeRuntimePreference(input.runtime);
  if (previousRuntime === nextRuntime) {
    return null;
  }

  const runtime = runtimes?.find((candidate) => candidate.id === nextRuntime);
  if (runtime?.availability === "available" && runtime.command) {
    return null;
  }

  const runtimeLabel = runtime?.label ?? "This provider";
  return `${runtimeLabel} is not available. Install it before saving this linked agent.`;
}

export async function submitProfilePersonaDialog({
  createManagedAgentForPersona,
  createPersona,
  hosted,
  input,
  managedAgent,
  onDone,
  previousPersona,
  runtimes,
  updateManagedAgent,
  updatePersona,
}: SubmitProfilePersonaDialogOptions) {
  let definitionSaved = false;
  try {
    if ("id" in input) {
      const runtimeEditError = validateLinkedAgentRuntimeEdit({
        input,
        managedAgent,
        previousPersona,
        runtimes,
      });
      if (runtimeEditError && !hosted) {
        toast.error(runtimeEditError);
        return;
      }

      const persona = await updatePersona(input);
      definitionSaved = true;
      const agentUpdate = managedAgent
        ? personaManagedAgentUpdate(managedAgent, persona, {
            previousPersona,
            runtimes: hosted
              ? hosted.runtime
                ? [hosted.runtime]
                : []
              : runtimes,
          })
        : null;
      const update =
        hosted && managedAgent
          ? {
              ...agentUpdate,
              pubkey: managedAgent.pubkey,
              installedRuntime: hosted.installedRuntime,
            }
          : agentUpdate;
      const result = update ? await updateManagedAgent(update) : null;
      if (result?.profileSyncError) {
        toast.warning(
          `${result.agent.name} was updated, but profile sync failed: ${result.profileSyncError}`,
        );
      }
      toast.success(
        hosted
          ? "Saved. Takes effect on next start."
          : `Updated ${input.displayName}.`,
      );
    } else {
      const persona = await createPersona(input);
      try {
        const created = await createManagedAgentForPersona(persona);
        if (created.spawnError) {
          toast.error(
            `${persona.displayName} was created, but it did not start: ${created.spawnError}`,
          );
        } else {
          toast.success(`Created and started ${created.agent.name}.`);
        }
        if (created.profileSyncError) {
          toast.warning(
            `${created.agent.name} was created, but profile sync failed: ${created.profileSyncError}`,
          );
        }
      } catch (error) {
        toast.error(
          error instanceof Error
            ? `${persona.displayName} was created, but the agent instance could not be created: ${error.message}`
            : `${persona.displayName} was created, but the agent instance could not be created.`,
        );
      }
    }

    onDone();
  } catch (error) {
    toast.error(
      `${definitionSaved ? "Definition saved; instance settings were not saved. " : ""}${error instanceof Error ? error.message : "Failed to save agent."}`,
    );
  }
}
