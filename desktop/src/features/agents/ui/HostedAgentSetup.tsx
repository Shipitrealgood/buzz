import * as React from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useRegisteredHostLabel } from "../lib/hostDirectory";
import {
  actOnHostedSetup,
  currentHostedModels,
  listHostedSetups,
  sameHostScope,
  type HostedAction,
  type HostedSetup,
  type HostScope,
} from "@/shared/api/hostedAgents";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "@/shared/ui/dialog";
import { truncateNpub } from "@/shared/lib/pubkey";
import { PersonaModelField } from "./PersonaModelField";

/** Shared continuation: finite explicit actions; never local login/start or polling. */
export function HostedAgentSetup({
  initial,
  onClose,
}: {
  initial: HostedSetup;
  onClose?: () => void;
}) {
  const [setup, setSetup] = React.useState(initial);
  const queryClient = useQueryClient();
  const cleanup = setup.action === "cleanup";
  const deleted = setup.state === "deleted";
  const hostLabel = useRegisteredHostLabel(initial.scope);
  const [model, setModel] = React.useState(initial.model ?? "");
  const [error, setError] = React.useState<string | null>(null);
  const [pending, setPending] = React.useState(false);
  const [now, setNow] = React.useState(Date.now());
  const generation = React.useRef(0);
  const latch = React.useRef(false);
  React.useEffect(
    () => () => {
      generation.current++;
    },
    [],
  );
  React.useEffect(() => {
    const deadline = setup.observation?.expires_at;
    if (!deadline) return;
    const timer = setTimeout(
      () => setNow(Date.now()),
      Math.max(0, deadline * 1000 - Date.now()) + 1,
    );
    return () => clearTimeout(timer);
  }, [setup.observation]);
  const catalog = currentHostedModels(setup, now);
  async function act(action: HostedAction) {
    if (latch.current) return;
    latch.current = true;
    const request = ++generation.current;
    setPending(true);
    setError(null);
    setSetup((old) => ({ ...old, observation: null }));
    try {
      const result = await actOnHostedSetup(setup, action);
      if (request !== generation.current) return;
      if (
        !sameHostScope(result.scope, initial.scope) ||
        result.pubkey !== initial.pubkey ||
        result.runtime !== initial.runtime
      )
        throw new Error("Setup scope changed.");
      setSetup(result);
      if (result.state === "deleted") {
        void queryClient.invalidateQueries({ queryKey: ["managed-agents"] });
      }
      setNow(Date.now());
    } catch (cause) {
      if (request === generation.current)
        setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      if (request === generation.current) {
        latch.current = false;
        setPending(false);
      }
    }
  }
  const labels: Record<string, string> = {
    unverified: "Check status to continue",
    unadmitted: "Admission pending",
    pending: "Preparing",
    "needs-login": "Host readiness and model setup",
    applied: "Deployment applied",
    failed: "Deployment failed",
    "cleanup-pending": "Host cleanup pending",
    "cleanup-failed": "Host cleanup failed; retry request",
    "cleanup-local-pending": "Host cleanup confirmed; local removal pending",
    deleted: "Agent deletion complete",
    conflict: "Placement conflict",
    unavailable: "Status unavailable",
  };
  const observation = setup.observation;
  return (
    <div className="space-y-4" data-testid="hosted-agent-setup">
      <p className="font-medium">
        {setup.name} · {truncateNpub(setup.pubkey)}
      </p>
      <p className="text-sm">
        Runs on {hostLabel ?? truncateNpub(setup.scope.host)}
      </p>
      <p role="status">{labels[setup.state] ?? setup.state}</p>
      {!cleanup ? (
        <>
          {observation ? (
            <div className="space-y-1 text-sm">
              <p>
                Host readiness:{" "}
                {observation.readiness === "not-applicable"
                  ? "Authentication not checked by this harness"
                  : (observation.readiness ?? "Unknown — refresh required")}
              </p>
              <p>Runtime: {observation.availability ?? "unknown"}</p>
              <p>
                Models:{" "}
                {observation.models.state === "unavailable"
                  ? observation.models.reason === "network-disabled"
                    ? "Networking is disabled on this host"
                    : observation.models.reason
                  : observation.models.state}
              </p>
            </div>
          ) : (
            <p className="text-sm">
              Readiness and models are pending or unavailable. Check status for
              current evidence.
            </p>
          )}
          <p className="text-sm">
            Install and configure the native harness on{" "}
            {hostLabel ?? "this host"} using its normal account and native
            sign-in command, then Refresh. A login check and model list do not
            establish subscription access or remaining usage.
          </p>
          <PersonaModelField
            disabled={pending || !catalog}
            isExplicitModelRequired
            model={model}
            modelSelectValue={model}
            modelDropdownOptions={(catalog?.models ?? []).map((m) => ({
              label: m.name ?? m.id,
              value: m.id,
            }))}
            modelDiscoveryStatus={
              !catalog
                ? {
                    tone: "warning",
                    message:
                      "Current host readiness and a fresh model list are required to start.",
                  }
                : null
            }
            onModelValueChange={setModel}
            onCustomModelChange={() => {}}
            showCustomModelInput={false}
            showSharedComputeAutoHint={false}
            transition={{ duration: 0 }}
          />
          {model && !catalog?.models.some((entry) => entry.id === model) ? (
            <p className="text-sm">Selected model: {model} (unavailable)</p>
          ) : null}
        </>
      ) : null}
      {error || setup.error ? (
        <p role="alert" className="text-sm text-destructive">
          {error ?? setup.error}
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          disabled={pending || deleted}
          onClick={() => void act({ type: "check" })}
        >
          Check status
        </Button>
        <Button
          type="button"
          disabled={pending || deleted}
          onClick={() => void act({ type: "retry" })}
        >
          Retry request
        </Button>
        {!cleanup ? (
          <>
            <Button
              type="button"
              disabled={
                pending ||
                !["needs-login", "failed", "unadmitted"].includes(setup.state)
              }
              onClick={() => void act({ type: "refresh" })}
            >
              Refresh
            </Button>
            <Button
              type="button"
              disabled={pending || !catalog?.models.some((m) => m.id === model)}
              onClick={() => void act({ type: "launch", model })}
            >
              Start
            </Button>
          </>
        ) : null}
        {!cleanup ? (
          <Button
            type="button"
            variant="destructive"
            disabled={pending}
            onClick={() => {
              if (
                window.confirm(
                  `Delete ${setup.name}? This removes the hosted agent after the host confirms cleanup.`,
                )
              )
                void act({ type: "cleanup" });
            }}
          >
            Delete agent
          </Button>
        ) : null}
        {onClose ? (
          <Button type="button" variant="outline" onClick={onClose}>
            Close
          </Button>
        ) : null}
      </div>
    </div>
  );
}

export function ContinueHostedSetup({
  scope,
  pubkey,
}: {
  scope: HostScope;
  pubkey: string;
}) {
  const [open, setOpen] = React.useState(false);
  const [setup, setSetup] = React.useState<HostedSetup | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setSetup(null);
    setError(null);
    void listHostedSetups(scope.community, scope.owner).then(
      (setups) => {
        if (cancelled) return;
        const found = setups.find(
          (s) => s.pubkey === pubkey && sameHostScope(s.scope, scope),
        );
        if (found) setSetup(found);
        else setError("No retained setup for this agent and workspace.");
      },
      (cause: unknown) => {
        if (!cancelled) setError(String(cause));
      },
    );
    return () => {
      cancelled = true;
    };
  }, [open, scope, pubkey]);
  return (
    <>
      <Button type="button" variant="outline" onClick={() => setOpen(true)}>
        Continue setup
      </Button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogTitle>Agent setup</DialogTitle>
          <DialogDescription>
            Continue this agent on its registered host.
          </DialogDescription>
          {setup ? (
            <HostedAgentSetup
              key={setup.operation}
              initial={setup}
              onClose={() => setOpen(false)}
            />
          ) : (
            <p role="status">{error ?? "Reading retained setup…"}</p>
          )}
        </DialogContent>
      </Dialog>
    </>
  );
}
