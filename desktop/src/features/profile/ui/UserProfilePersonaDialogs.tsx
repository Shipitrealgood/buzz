import * as React from "react";
import { skipToken, useQuery } from "@tanstack/react-query";
import {
  hostedCatalog,
  listHostedSetups,
  sameHostScope,
} from "@/shared/api/hostedAgents";
import { AgentRunLocationProvider } from "@/features/agents/ui/AgentRunLocationContext";
import { runLocationForBackend } from "@/features/agents/lib/agentAccessWarning";
import type { HostedEditSelection } from "./UserProfilePanelPersonaSubmit";

import type {
  AcpRuntimeCatalogEntry,
  AgentPersona,
  CreatePersonaInput,
  ManagedAgent,
  UpdatePersonaInput,
} from "@/shared/api/types";
import { AgentCardMintDialog } from "@/features/agents/ui/AgentCardMintDialog";
import { PersonaDeleteDialog } from "@/features/agents/ui/PersonaDeleteDialog";
import { AgentDefinitionDialog } from "@/features/agents/ui/AgentDefinitionDialog";
import type { PersonaDialogState } from "@/features/agents/ui/personaDialogState";
import { UserProfileSnapshotExportDialog } from "@/features/profile/ui/UserProfileSnapshotExportDialog";

/** Agent selected for card minting. */
export type CardMintTarget = {
  id: string;
  name: string;
  canLock: boolean;
};

/**
 * Card-mint dialog state plus the callback that opens it. `create` is
 * undefined when no persona resolves; owner gating is the caller's job.
 */
export function useCardMint(
  persona: AgentPersona | undefined,
  managedAgent: ManagedAgent | undefined,
) {
  const [target, setTarget] = React.useState<CardMintTarget | null>(null);
  const close = React.useCallback(() => setTarget(null), []);
  const create = persona
    ? () =>
        setTarget({
          // Prefer the live instance pubkey; fall back to the
          // persona/definition id (same resolution as export).
          id: managedAgent?.pubkey ?? persona.id,
          name: persona.displayName,
          // Locking needs an instance keypair to encrypt to.
          canLock: Boolean(managedAgent?.pubkey),
        })
    : undefined;
  return { close, create, target };
}

export function UserProfilePersonaDialogs({
  cardMintTarget,
  createError,
  instanceCount,
  isPending,
  linkedAgentPubkey,
  managedAgent,
  personaDialogState,
  personaToDelete,
  personaToExportSnapshot,
  resolvedPersona,
  runtimes,
  runtimesLoading,
  runtimesError = false,
  updateError,
  onCloseCardMint,
  onCloseDelete,
  onCloseDialog,
  onCloseExportSnapshot,
  onConfirmDelete,
  onExportSnapshot,
  onSubmit,
}: {
  cardMintTarget: CardMintTarget | null;
  createError: Error | null;
  /** Number of managed-agent instances backed by the persona being deleted. */
  instanceCount: number;
  isPending: boolean;
  linkedAgentPubkey: string | null;
  managedAgent?: ManagedAgent;
  personaDialogState: PersonaDialogState | null;
  personaToDelete: AgentPersona | null;
  personaToExportSnapshot: AgentPersona | null;
  resolvedPersona: AgentPersona | undefined;
  runtimes: AcpRuntimeCatalogEntry[];
  runtimesLoading: boolean;
  runtimesError?: boolean;
  updateError: Error | null;
  onCloseCardMint: () => void;
  onCloseDelete: () => void;
  onCloseDialog: () => void;
  onCloseExportSnapshot: () => void;
  onConfirmDelete: (persona: AgentPersona) => void;
  onExportSnapshot: (persona: AgentPersona) => void;
  onSubmit: (
    input: CreatePersonaInput | UpdatePersonaInput,
    hosted?: HostedEditSelection,
  ) => Promise<void>;
}) {
  const backend = managedAgent?.backend;
  const scope = backend?.type === "registered_host" ? backend.scope : undefined;
  const hosted = !!scope;
  const active = hosted && !!personaDialogState;
  const catalog = useQuery({
    queryKey: ["hosted-edit-catalog", scope],
    queryFn: scope ? () => hostedCatalog(scope) : skipToken,
    enabled: active,
    retry: false,
    refetchOnWindowFocus: false,
  });
  const retained = useQuery({
    queryKey: ["hosted-edit-context", scope, linkedAgentPubkey],
    queryFn: scope
      ? () => listHostedSetups(scope.community, scope.owner)
      : skipToken,
    enabled:
      active &&
      backend?.type === "registered_host" &&
      !backend.installed_runtime,
    retry: false,
  });
  const contexts = retained.data?.filter(
    (item) =>
      scope &&
      item.pubkey === linkedAgentPubkey &&
      sameHostScope(item.scope, scope),
  );
  const installed =
    backend?.type === "registered_host"
      ? (backend.installed_runtime ??
        (contexts?.length === 1 ? contexts[0].runtime : undefined))
      : undefined;
  const initialValues = React.useMemo(() => {
    const values = personaDialogState?.initialValues;
    return values && hosted
      ? { ...values, runtime: installed ?? "" }
      : (values ?? null);
  }, [personaDialogState, installed, hosted]);
  const hostRuntimes =
    catalog.data?.runtimes.flatMap(({ installedId, metadata }) =>
      metadata
        ? [
            {
              ...metadata,
              id: installedId,
              label: `${metadata.label} · ${installedId}`,
            },
          ]
        : [],
    ) ?? [];
  const runtimeCatalogStatus = scope
    ? catalog.isPending
      ? "loading"
      : catalog.isError
        ? "error"
        : "ready"
    : runtimesLoading
      ? "loading"
      : runtimesError
        ? "error"
        : "ready";
  return (
    <>
      <AgentRunLocationProvider
        runLocation={
          managedAgent ? runLocationForBackend(managedAgent.backend) : null
        }
      >
        <AgentDefinitionDialog
          configurationSource={
            scope
              ? {
                  type: "hosted",
                  key: JSON.stringify(scope),
                  scope,
                  catalogRevision:
                    catalog.data?.status.capability.catalog_revision,
                }
              : undefined
          }
          description={`${personaDialogState?.description ?? ""}${scope ? " Settings are saved for this host and take effect on next start." : ""}`}
          error={
            scope && !installed
              ? new Error(
                  "Exact installed host runtime is missing; retained context is required.",
                )
              : (updateError ?? createError)
          }
          initialValues={initialValues}
          isPending={isPending}
          runtimes={scope ? hostRuntimes : runtimes}
          runtimeCatalogStatus={runtimeCatalogStatus}
          onOpenChange={(open) => {
            if (!open) {
              onCloseDialog();
            }
          }}
          onSubmit={async (input) => {
            if (!scope) return onSubmit(input);
            if (!installed || !input.runtime) return;
            const chosen = catalog.data?.runtimes.find(
              (item) => item.installedId === input.runtime,
            )?.metadata;
            const canonical =
              input.runtime === installed
                ? personaDialogState?.initialValues.runtime
                : chosen?.id;
            if (!canonical) return;
            return onSubmit(
              { ...input, runtime: canonical },
              { installedRuntime: input.runtime, runtime: chosen },
            );
          }}
          open={personaDialogState !== null}
          submitLabel={personaDialogState?.submitLabel ?? "Save"}
          title={personaDialogState?.title ?? "Agent"}
        />
      </AgentRunLocationProvider>
      <PersonaDeleteDialog
        instanceCount={instanceCount}
        onConfirm={onConfirmDelete}
        onOpenChange={(open) => {
          if (!open) {
            onCloseDelete();
          }
        }}
        open={personaToDelete !== null}
        persona={personaToDelete}
      />
      {personaToExportSnapshot ? (
        <UserProfileSnapshotExportDialog
          linkedAgentPubkey={linkedAgentPubkey}
          onOpenChange={(open) => {
            if (!open) onCloseExportSnapshot();
          }}
          persona={personaToExportSnapshot}
        />
      ) : null}
      {cardMintTarget ? (
        <AgentCardMintDialog
          agentId={cardMintTarget.id}
          agentName={cardMintTarget.name}
          canLock={cardMintTarget.canLock}
          onExportInstead={
            resolvedPersona
              ? () => {
                  // Free path: swap the mint dialog for the ordinary
                  // snapshot export flow (same importable agent, no spend).
                  onCloseCardMint();
                  onExportSnapshot(resolvedPersona);
                }
              : undefined
          }
          onOpenChange={(open) => {
            if (!open) onCloseCardMint();
          }}
        />
      ) : null}
    </>
  );
}
