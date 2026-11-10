import {
  hostedCatalog,
  hostedSetupToken,
  listHostedSetups,
  currentHostedCatalog,
  sameHostScope,
  type HostedCatalog,
  type HostedSetup,
} from "@/shared/api/hostedAgents";
import type {
  HostedDraft,
  AgentSubmitResult,
} from "../lib/prepareHostedDefinition";
import { useCommunities } from "@/features/communities/useCommunities";
import { useIdentityQuery } from "@/shared/api/hooks";
import { isWindowsPlatform } from "@/shared/lib/platform";
import { Button } from "@/shared/ui/button";
import { AgentDefinitionDialogShell } from "./AgentDefinitionDialogShell";
import { HostedAgentSetup } from "./HostedAgentSetup";
import * as React from "react";

import type {
  AcpRuntimeCatalogEntry,
  AcpRuntime,
  CreatePersonaInput,
  ManagedAgent,
  UpdatePersonaInput,
} from "@/shared/api/types";
import {
  runLocationForBackend,
  runLocationForRunOn,
} from "../lib/agentAccessWarning";
import { AgentRunLocationProvider } from "./AgentRunLocationContext";
import type { BackendIntent } from "../lib/instanceInputForDefinition";
import type { AgentCreateIntent } from "./agentCreateIntent";
import type { EditAgentFocusTarget } from "@/features/agents/openEditAgentEvent";
import { AgentInstanceEditDialog } from "./AgentInstanceEditDialog";
import { createPersonaDialogState } from "./personaDialogState";
import {
  AgentDefinitionDialog,
  type AgentDefinitionSubmitOptions,
} from "./AgentDefinitionDialog";
import { WhereToRunSection } from "./WhereToRunSection";
import { EffortPickerField } from "./EffortPickerField";
import { isSavableEffort } from "./effortPicker";
import {
  canSubmitWhereToRun,
  emptyWhereToRunDraft,
  resolveBackendIntent,
} from "./whereToRunIntent";

type AgentDialogCreateProps = {
  mode: "definition";
  embedded?: boolean;
  submitLabel?: string;
  initialValues?: CreatePersonaInput | null;
  onDirtyChange?: (dirty: boolean) => void;
  onOpenChange: (open: boolean) => void;
  definitionError: Error | null;
  isDefinitionPending: boolean;
  runtimes: AcpRuntimeCatalogEntry[];
  runtimeCatalogStatus: "loading" | "ready" | "error";
  onSubmitDefinition: (
    input: CreatePersonaInput | UpdatePersonaInput,
    intent: AgentCreateIntent,
    backendIntent: BackendIntent | null,
    effortLevel: string | null,
  ) => Promise<AgentSubmitResult>;
};

type AgentDialogInstanceEditProps = {
  mode: "instance-edit";
  agent: ManagedAgent;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onUpdated?: (agent: ManagedAgent) => void;
  initialFocus?: EditAgentFocusTarget;
  /**
   * Called when the user clicks "Edit avatar" inside the instance-edit dialog.
   * Caller (UserProfilePanel) is responsible for closing this dialog and
   * opening the definition-edit dialog. Only passed when the linked definition
   * is editable (non-built-in, resolved).
   */
  onEditLinkedPersona?: () => void;
};

type AgentDialogDefinitionEditProps = {
  mode: "definition-edit";
  open: boolean;
  title: string;
  description: string;
  submitLabel: string;
  initialValues: CreatePersonaInput | UpdatePersonaInput | null;
  error: Error | null;
  isPending: boolean;
  runtimes: AcpRuntimeCatalogEntry[];
  runtimeCatalogStatus?: "loading" | "ready" | "error";
  onOpenChange: (open: boolean) => void;
  onSubmit: (
    input: CreatePersonaInput | UpdatePersonaInput,
    options: AgentDefinitionSubmitOptions,
  ) => Promise<unknown>;
  publishCatalogUpdatesOnSave?: boolean;
};

type AgentDialogProps =
  | AgentDialogCreateProps
  | AgentDialogInstanceEditProps
  | AgentDialogDefinitionEditProps;

/**
 * Unified entry point (Phase 1B.2/1B.3b/1B.3c): routes an intent to the form
 * that owns it. The definition family renders AgentDefinitionDialog — create
 * mode always starts the agent and includes a WhereToRunSection;
 * definition-edit passes the caller's PersonaDialogState-derived props
 * through unchanged (edit/duplicate/import). instance-edit renders
 * AgentInstanceEditDialog (persistent mount + `open` toggle — its reset
 * lifecycle is keyed on [open, agent.pubkey]).
 */
export function AgentDialog(props: AgentDialogProps) {
  if (props.mode === "instance-edit") {
    return (
      // A running instance knows its own backend, so the respond-to warning can
      // name the machine it will actually run on.
      <AgentRunLocationProvider
        runLocation={runLocationForBackend(props.agent.backend)}
      >
        <AgentInstanceEditDialog
          agent={props.agent}
          onEditLinkedPersona={props.onEditLinkedPersona}
          onOpenChange={props.onOpenChange}
          onUpdated={props.onUpdated}
          open={props.open}
          initialFocus={props.initialFocus}
        />
      </AgentRunLocationProvider>
    );
  }
  if (props.mode === "definition-edit") {
    // A definition has no instance and no run draft, so the run location stays
    // unknown and the warning uses its local-wording fallback.
    const { mode: _mode, ...definitionProps } = props;
    return <AgentDefinitionDialog {...definitionProps} />;
  }
  return <AgentCreateDialogRouter {...props} />;
}

function AgentCreateDialogRouter({
  embedded,
  initialValues: providedInitialValues,
  onOpenChange,
  definitionError,
  isDefinitionPending,
  runtimes,
  runtimeCatalogStatus,
  submitLabel,
  onDirtyChange,
  onSubmitDefinition,
}: AgentDialogCreateProps) {
  const [runDraft, setRunDraft] = React.useState(emptyWhereToRunDraft);
  const [effortLevel, setEffortLevel] = React.useState<string | null>(null);
  const relay = useCommunities().activeCommunity?.relayUrl;
  const owner = useIdentityQuery().data?.pubkey;
  const scope = runDraft.destination?.scope;
  const scopeKey = scope ? JSON.stringify(scope) : "local";
  const [catalog, setCatalog] = React.useState<HostedCatalog | null>(null);
  const [catalogError, setCatalogError] = React.useState<string | null>(null);
  const [token, setToken] = React.useState<string | null>(null);
  const tokenRequest = React.useRef<Promise<string> | null>(null);
  const [setup, setSetup] = React.useState<HostedSetup | null>(null);
  const [retained, setRetained] = React.useState<HostedSetup[]>([]);
  const [submitError, setSubmitError] = React.useState<string | null>(null);
  const [now, setNow] = React.useState(Date.now());
  const submitLatch = React.useRef(false);
  const draft = React.useRef<HostedDraft | null>(null);
  const generation = React.useRef(0);
  const current = React.useRef({ scopeKey, alive: true });
  current.current.scopeKey = scopeKey;
  React.useEffect(() => {
    current.current.alive = true;
    return () => {
      generation.current++;
      current.current.alive = false;
    };
  }, []);
  React.useEffect(() => {
    if (!isWindowsPlatform()) return;
    let cancelled = false;
    tokenRequest.current ??= hostedSetupToken();
    void tokenRequest.current.then(
      (value) => {
        if (!cancelled) setToken(value);
      },
      (cause: unknown) => {
        if (!cancelled) setCatalogError(String(cause));
      },
    );
    return () => {
      cancelled = true;
    };
  }, []);
  React.useEffect(() => {
    if (!isWindowsPlatform() || !relay || !owner) return;
    let cancelled = false;
    const origin = relay.replace(/^wss:/, "https:").replace(/\/$/, "");
    void listHostedSetups(origin, owner).then(
      (items) => {
        if (!cancelled)
          setRetained(
            items.filter(
              (item) =>
                item.scope.community === origin && item.scope.owner === owner,
            ),
          );
      },
      () => {},
    );
    return () => {
      cancelled = true;
    };
  }, [relay, owner]);
  React.useEffect(() => {
    generation.current++;
    setCatalog(null);
    setCatalogError(null);
    if (!scope) return;
    let cancelled = false;
    void hostedCatalog(scope).then(
      (value) => {
        if (!cancelled && sameHostScope(value.status.scope, scope)) {
          setCatalog(value);
          setNow(Date.now());
        }
      },
      (cause: unknown) => {
        if (!cancelled) setCatalogError(String(cause));
      },
    );
    return () => {
      cancelled = true;
    };
  }, [scope]);
  React.useEffect(() => {
    if (!catalog) return;
    const timer = setTimeout(
      () => setNow(Date.now()),
      Math.max(0, catalog.status.expires_at * 1000 - Date.now()) + 1,
    );
    return () => clearTimeout(timer);
  }, [catalog]);
  const hostedReady =
    !!scope && !!token && currentHostedCatalog(catalog, scope, now);
  const hostRuntimes = React.useMemo(
    () =>
      (catalog?.runtimes ?? []).flatMap(({ installedId, metadata }) =>
        metadata
          ? [
              {
                ...metadata,
                id: installedId,
                label: `${metadata.label} · ${installedId}`,
              },
            ]
          : [],
      ),
    [catalog],
  );
  const initialValues = React.useMemo(
    () => providedInitialValues ?? createPersonaDialogState().initialValues,
    [providedInitialValues],
  );

  const copy = createPersonaDialogState();
  // Effort is local-only: a provider backend sets it at deploy time.
  const backendIntent = resolveBackendIntent(runDraft);

  if (setup)
    return (
      <AgentDefinitionDialogShell
        embedded={embedded ?? false}
        open
        title="Agent setup"
        description="Continue this agent on its registered host."
        onOpenChange={onOpenChange}
        footer={null}
      >
        <HostedAgentSetup
          key={setup.operation}
          initial={setup}
          onClose={() => onOpenChange(false)}
        />
      </AgentDefinitionDialogShell>
    );

  return (
    // The create flow is the one surface that knows where the agent will run,
    // because it owns the "Run on" draft.
    <AgentRunLocationProvider runLocation={runLocationForRunOn(runDraft.runOn)}>
      <AgentDefinitionDialog
        configurationSource={
          scope ? { type: "hosted", key: scopeKey } : undefined
        }
        createRunSection={(effortOptions) => (
          <>
            <WhereToRunSection
              draft={runDraft}
              isPending={isDefinitionPending}
              onDraftChange={(nextDraft) => {
                setRunDraft(nextDraft);
                onDirtyChange?.(true);
              }}
            />
            {scope ? (
              <p role="status" className="text-sm">
                {catalogError ??
                  (!catalog
                    ? "Reading host runtime metadata…"
                    : !hostedReady
                      ? "Host metadata is expired or unavailable. Reselect the host to refresh."
                      : !hostRuntimes.length
                        ? "No supported installed harnesses on this host."
                        : "Choose an installed host harness, then prepare this agent.")}
              </p>
            ) : null}
            {retained.map((item) => (
              <Button
                key={item.operation}
                type="button"
                variant="outline"
                onClick={() => setSetup(item)}
              >
                Continue setup: {item.name}
              </Button>
            ))}
            {backendIntent ? null : (
              <EffortPickerField
                backend={{ type: "local" }}
                choices={effortOptions}
                disabled={isDefinitionPending}
                onChange={setEffortLevel}
                value={effortLevel}
              />
            )}
          </>
        )}
        createSubmitBlocked={!canSubmitWhereToRun(runDraft, hostedReady)}
        description={copy.description}
        embedded={embedded}
        error={submitError ? new Error(submitError) : definitionError}
        initialValues={initialValues}
        isPending={isDefinitionPending}
        onDirtyChange={onDirtyChange}
        onOpenChange={onOpenChange}
        onSubmit={async (input, { effortOptions }) => {
          if (
            submitLatch.current ||
            !canSubmitWhereToRun(runDraft, hostedReady)
          )
            return;
          submitLatch.current = true;
          const request = generation.current;
          setSubmitError(null);
          try {
            let destination = backendIntent;
            if (
              draft.current?.frozen &&
              (destination?.type !== "registered-host" ||
                input.runtime !== draft.current.runtime ||
                !sameHostScope(destination.scope, draft.current.scope))
            ) {
              throw new Error(
                `This setup is fixed to ${draft.current.runtime} on its original host. Restore that selection to retry, or close and reopen Continue setup.`,
              );
            }
            if (destination?.type === "registered-host") {
              const chosen = catalog?.runtimes.find(
                (item) => item.installedId === input.runtime,
              );
              if (
                !token ||
                !chosen?.metadata ||
                chosen.metadata.availability !== "available" ||
                !chosen.metadata.command
              )
                throw new Error("Choose a supported installed host harness.");
              if (!draft.current)
                draft.current = {
                  operation: token,
                  runtime: chosen.installedId,
                  scope: destination.scope,
                  harness: chosen.metadata as AcpRuntime,
                  isCurrent: () =>
                    current.current.alive &&
                    current.current.scopeKey === scopeKey &&
                    Date.now() < (catalog?.status.expires_at ?? 0) * 1000,
                };
              if (!draft.current.frozen) {
                draft.current.runtime = chosen.installedId;
                draft.current.scope = destination.scope;
                draft.current.harness = chosen.metadata as AcpRuntime;
              }
              draft.current.isCurrent = () =>
                current.current.alive &&
                current.current.scopeKey === scopeKey &&
                Date.now() < (catalog?.status.expires_at ?? 0) * 1000;
              destination = { ...destination, setup: draft.current };
            }
            const submitted = await onSubmitDefinition(
              input,
              "definition_start",
              destination,
              !backendIntent && isSavableEffort(effortLevel, effortOptions)
                ? effortLevel
                : null,
            );
            if (!current.current.alive || request !== generation.current)
              return;
            if (typeof submitted === "object") {
              setSetup(submitted.hosted);
              onDirtyChange?.(false);
            } else if (submitted) {
              onDirtyChange?.(false);
              onOpenChange(false);
            } else if (scope)
              setSubmitError(
                "Setup could not complete. Retry this draft or reopen its retained setup.",
              );
          } catch (cause) {
            if (request === generation.current)
              setSubmitError(
                cause instanceof Error ? cause.message : String(cause),
              );
          } finally {
            submitLatch.current = false;
          }
        }}
        open
        runtimes={scope ? hostRuntimes : runtimes}
        runtimeCatalogStatus={
          scope
            ? catalogError
              ? "error"
              : catalog
                ? "ready"
                : "loading"
            : runtimeCatalogStatus
        }
        submitLabel={
          scope ? "Prepare agent" : (submitLabel ?? copy.submitLabel)
        }
        title={copy.title}
      />
    </AgentRunLocationProvider>
  );
}
