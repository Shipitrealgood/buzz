import { AlertTriangle } from "lucide-react";
import * as React from "react";

import { useBackendProvidersQuery } from "@/features/agents/hooks";
import { useCommunities } from "@/features/communities/useCommunities";
import { useIdentityQuery } from "@/shared/api/hooks";
import type { BackendProviderCandidate } from "@/shared/api/types";
import { useHostDirectory, type Session } from "../lib/hostDirectory";
import {
  hostBelongsToSession,
  hostDestination,
  hostDestinationValue,
  hostRegistrationStatus,
} from "../lib/hostDestination";
import { isWindowsPlatform } from "@/shared/lib/platform";
import { truncateNpub } from "@/shared/lib/pubkey";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "@/shared/ui/dialog";
import { MyHosts } from "./PairedHostsSection";
import { probeBackendProvider } from "@/shared/api/tauri";

import { ProviderConfigFields } from "./ProviderConfigFields";
import { PersonaDropdownField } from "./PersonaDropdownField";
import {
  applyProbeResult,
  emptyWhereToRunDraft,
  type WhereToRunDraft,
} from "./whereToRunIntent";

type WhereToRunProps = {
  draft: WhereToRunDraft;
  isPending: boolean;
  onDraftChange: (next: WhereToRunDraft) => void;
};

/** Shared local/provider/registered-host destination selector. */
export function WhereToRunSection(props: WhereToRunProps) {
  const backendProviders = useBackendProvidersQuery().data ?? [];
  const relay = useCommunities().activeCommunity?.relayUrl;
  const owner = useIdentityQuery().data?.pubkey;
  const session =
    isWindowsPlatform() && relay && owner
      ? { expectedRelay: relay, expectedOwner: owner }
      : null;
  if (!isWindowsPlatform() && backendProviders.length === 0) return null;
  return (
    <WhereToRunFields
      {...props}
      backendProviders={backendProviders}
      session={session}
    />
  );
}

/** Query and rendering seam shared by the actual Add Agent flow and UI tests. */
export function WhereToRunFields({
  draft,
  isPending,
  onDraftChange,
  backendProviders,
  session,
}: WhereToRunProps & {
  backendProviders: BackendProviderCandidate[];
  session: Session | null;
}) {
  const directory = useHostDirectory(session);
  const [showHosts, setShowHosts] = React.useState(false);
  const hosts = (directory.data?.hosts ?? []).filter(
    (host) => session && hostBelongsToSession(host, session),
  );
  const isHostMode =
    Boolean(draft.destination) || draft.runOn.startsWith("host:");
  const selectedHost = hosts.find(
    (host) => hostDestinationValue(host) === draft.runOn,
  );
  const directoryCurrent =
    !directory.error &&
    directory.data?.supported === true &&
    directory.data.available === true;
  const registrationStatus = (host: (typeof hosts)[number]) =>
    host.state === "paired"
      ? directoryCurrent
        ? "Registered"
        : "Saved registration · unavailable"
      : hostRegistrationStatus(host);
  const hostAvailable =
    !!selectedHost && selectedHost.state === "paired" && directoryCurrent;
  React.useEffect(() => {
    if (isHostMode && draft.hostAvailable !== hostAvailable)
      onDraftChange({ ...draft, hostAvailable });
  }, [isHostMode, hostAvailable, draft, onDraftChange]);
  const [probeError, setProbeError] = React.useState<string | null>(null);
  const runOnOptions = [
    { label: "This computer", value: "local" },
    ...backendProviders.map((provider) => ({
      label: provider.id,
      value: provider.id,
    })),
    ...hosts.map((host) => ({
      label: `${host.label ?? "Linux host"} · ${truncateNpub(host.scope.host)} · ${registrationStatus(host)}`,
      value: hostDestinationValue(host),
    })),
  ];
  const isProviderMode = !isHostMode && draft.runOn !== "local";
  const selectedBackendProvider = React.useMemo(
    () =>
      backendProviders.find((provider) => provider.id === draft.runOn) ?? null,
    [backendProviders, draft.runOn],
  );

  // Latest-state seam for probe resolution: an Effect Event always sees the
  // draft as it is *now*. Without this, the probe promise closes over the
  // draft from probe start, and anything typed while the probe was in flight
  // gets thrown away when it resolves (a second, subtler Typewriter Eraser).
  const applyProbe = React.useEffectEvent(
    (result: Awaited<ReturnType<typeof probeBackendProvider>>) => {
      onDraftChange(applyProbeResult(draft, result));
    },
  );

  // Probe once per provider *selection*, keyed on the provider's stable
  // path — never on the draft. Depending on the draft made every keystroke
  // refire the probe, and each resolution reset providerConfig to schema
  // defaults, which erased what the user was typing (the Typewriter Eraser)
  // and spawned the provider binary in a loop for as long as the dialog was
  // open. Keying on the path (not the provider object) also keeps a
  // providers-query refresh from reprobing an unchanged selection.
  const selectedBinaryPath = isProviderMode
    ? (selectedBackendProvider?.binaryPath ?? null)
    : null;
  React.useEffect(() => {
    if (!selectedBinaryPath || draft.probedProvider) {
      setProbeError(null);
      return;
    }
    let cancelled = false;
    setProbeError(null);
    void probeBackendProvider(selectedBinaryPath)
      .then((result) => {
        if (cancelled) return;
        applyProbe(result);
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setProbeError(error instanceof Error ? error.message : String(error));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [selectedBinaryPath, draft.probedProvider]);

  return (
    <div className="space-y-4">
      <div className="space-y-1.5">
        <label className="text-sm font-medium" htmlFor="agent-run-on">
          Run on
        </label>
        <PersonaDropdownField
          disabled={isPending}
          id="agent-run-on"
          onValueChange={(runOn) => {
            const host = hosts.find(
              (item) => hostDestinationValue(item) === runOn,
            );
            onDraftChange({
              ...emptyWhereToRunDraft,
              runOn,
              ...(host && { destination: hostDestination(host) }),
            });
          }}
          options={runOnOptions}
          placeholder="Choose where to run"
          value={draft.runOn}
        />
      </div>

      {isWindowsPlatform() && (
        <div className="space-y-2 text-sm text-muted-foreground">
          {!session ? (
            <p>
              Select a community and unlock your identity to see registered
              hosts.
            </p>
          ) : directory.isPending ? (
            <p>Checking host registrations…</p>
          ) : directory.error || !directory.data?.available ? (
            <p>
              Host registrations unavailable. Saved information may be out of
              date.
            </p>
          ) : !directory.data.supported ? (
            <p>Host registration is not supported by this community.</p>
          ) : !hosts.length ? (
            <p>No registered hosts.</p>
          ) : null}
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isPending || !session}
            onClick={() => setShowHosts(true)}
          >
            Add Host
          </Button>
          {isHostMode ? (
            <p role="status">
              {selectedHost
                ? registrationStatus(selectedHost)
                : "Selected host is unavailable in this workspace."}{" "}
              Host runtime metadata is checked separately. Models remain pending
              until this new agent signs in on its host.
            </p>
          ) : null}
        </div>
      )}
      <Dialog open={showHosts} onOpenChange={setShowHosts}>
        <DialogContent onSubmit={(event) => event.stopPropagation()}>
          <DialogTitle>Add Host</DialogTitle>
          <DialogDescription>
            Register a host using its private invitation.
          </DialogDescription>
          {session ? (
            <MyHosts
              key={session.expectedRelay + session.expectedOwner}
              session={session}
            />
          ) : null}
        </DialogContent>
      </Dialog>

      {isProviderMode && selectedBackendProvider ? (
        <div className="space-y-4">
          <div className="flex gap-3 rounded-2xl border border-warning/30 bg-warning-bg px-4 py-3">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-warning" />
            <p className="text-sm text-warning">
              This provider at{" "}
              <span className="font-mono font-medium">
                {selectedBackendProvider.binaryPath}
              </span>{" "}
              will receive your agent&apos;s private key. Only use providers
              from trusted sources.
            </p>
          </div>
          {probeError ? (
            <p className="rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              Could not probe provider: {probeError}
            </p>
          ) : null}
          {draft.probedProvider?.config_schema ? (
            <ProviderConfigFields
              config={draft.providerConfig}
              onChange={(providerConfig) =>
                onDraftChange({ ...draft, providerConfig })
              }
              schema={draft.probedProvider.config_schema}
            />
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
