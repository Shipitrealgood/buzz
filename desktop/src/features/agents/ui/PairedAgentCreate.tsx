import * as React from "react";
import { useChannelsQuery } from "@/features/channels/hooks";
import { useAgentAccessOwnerOnlyQuery } from "@/features/agents/useAgentAccessOwnerOnly";
import { invokeTauri } from "@/shared/api/tauri";
import type { RespondToMode } from "@/shared/api/types";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import {
  CreateAgentRespondToField,
  OWNER_ONLY_ACCESS_DISABLED_REASON,
} from "./RespondToField";
import type { Host, Session, HostRequest } from "./PairedHostsSection";

export function PairedAgentCreate({
  current,
  session,
  reload,
  close,
}: {
  current: Host;
  session: Session;
  reload: () => void;
  close: () => void;
}) {
  const id = React.useId();
  const [name, setName] = React.useState("");
  const [runtime, setRuntime] = React.useState(current.runtimes[0] ?? "");
  const [mode, setMode] = React.useState<RespondToMode>("owner-only");
  const [allow, setAllow] = React.useState<string[]>([]);
  const [channelPolicy, setChannelPolicy] = React.useState("owner_only");
  const [selected, setSelected] = React.useState<string[]>([]);
  const [busy, setBusy] = React.useState(false);
  const inFlight = React.useRef(false);
  const [error, setError] = React.useState("");
  const [frozen, setFrozen] = React.useState<Record<string, unknown> | null>(
    null,
  );
  const [revision] = React.useState(current.catalog_revision);
  const channels = useChannelsQuery();
  const access = useAgentAccessOwnerOnlyQuery();
  const cannotShare = access.data !== false;
  const available = (channels.data ?? []).filter(
    (c) =>
      !c.archivedAt &&
      (c.channelType === "stream" || c.channelType === "forum"),
  );
  const catalogChanged = revision !== current.catalog_revision;
  const valid =
    current.fresh &&
    current.inventory_id &&
    !catalogChanged &&
    current.runtimes.includes(runtime) &&
    name.trim() &&
    allow.length <= 64 &&
    (mode !== "allowlist" || allow.length > 0) &&
    (!cannotShare || mode === "owner-only") &&
    selected.length <= 8 &&
    selected.every((key) => available.some((c) => c.id === key)) &&
    (!selected.length || channelPolicy !== "nobody");
  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (inFlight.current || (!frozen && !valid)) return;
    inFlight.current = true;
    setBusy(true);
    setError("");
    const tap = Array.from(crypto.getRandomValues(new Uint8Array(32)), (b) =>
      b.toString(16).padStart(2, "0"),
    ).join("");
    const payload = frozen ?? {
      ...session,
      host: current.scope.host,
      generation: current.scope.generation,
      catalogId: current.inventory_id,
      tap,
      draft: {
        name: name.trim().normalize("NFC"),
        runtime,
        channels: [...selected].sort(),
        policy: {
          respond_to: mode,
          respond_to_allowlist:
            mode === "allowlist" ? [...new Set(allow)].sort() : [],
          channel_add_policy: channelPolicy,
        },
      },
    };
    setFrozen(payload);
    try {
      await invokeTauri<HostRequest>("paired_host_create", payload);
      close();
    } catch (cause) {
      setError(
        cause instanceof Error
          ? cause.message
          : "Submission was interrupted. Retry the same request.",
      );
    } finally {
      inFlight.current = false;
      setBusy(false);
      reload();
    }
  }
  return (
    <form
      className="space-y-3 rounded-md border p-3"
      aria-label="Create an agent on this host"
      onSubmit={submit}
    >
      <p className="text-sm font-medium">
        Create on {current.label ?? "selected host"}
      </p>
      <fieldset className="space-y-2" disabled={Boolean(frozen) || busy}>
        <label htmlFor={`${id}-name`} className="text-sm">
          Agent name
        </label>
        <Input
          id={`${id}-name`}
          value={name}
          required
          onChange={(e) => setName(e.target.value)}
        />
        <label htmlFor={`${id}-runtime`} className="block text-sm">
          Installed runtime
        </label>
        <select
          id={`${id}-runtime`}
          value={runtime}
          required
          onChange={(e) => setRuntime(e.target.value)}
        >
          {!current.runtimes.includes(runtime) && (
            <option value={runtime}>Select an installed runtime</option>
          )}
          {current.runtimes.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>
        <CreateAgentRespondToField
          mode={mode}
          allowlist={allow}
          onModeChange={setMode}
          onAllowlistChange={setAllow}
          ownerPubkey={session.expectedOwner}
          runLocation="remote"
          disabled={cannotShare}
          disabledReason={
            cannotShare ? OWNER_ONLY_ACCESS_DISABLED_REASON : undefined
          }
        />
        <p className="text-xs text-muted-foreground">
          The community can read the agent name, runtime, response policy and
          any selected people's public keys.
        </p>
        <label htmlFor={`${id}-add`} className="block text-sm">
          Who can add this agent to channels?
        </label>
        <select
          id={`${id}-add`}
          value={channelPolicy}
          onChange={(e) => setChannelPolicy(e.target.value)}
        >
          <option value="owner_only">Only me</option>
          <option value="anyone">Anyone allowed by the channel</option>
          <option value="nobody">Nobody</option>
        </select>
        <fieldset className="space-y-1">
          <legend className="text-sm">Initial channels (up to eight)</legend>
          {channels.isLoading && <p>Loading channels…</p>}
          {channels.error && <p role="alert">Channels could not be loaded.</p>}
          {available.map((c) => (
            <label className="flex gap-2" key={c.id}>
              <input
                type="checkbox"
                checked={selected.includes(c.id)}
                disabled={!selected.includes(c.id) && selected.length >= 8}
                onChange={(e) =>
                  setSelected((old) =>
                    e.target.checked
                      ? [...old, c.id]
                      : old.filter((k) => k !== c.id),
                  )
                }
              />
              {c.name}
            </label>
          ))}
          {selected
            .filter((key) => !available.some((c) => c.id === key))
            .map((key) => (
              <label className="flex gap-2" key={key}>
                <input
                  type="checkbox"
                  checked
                  onChange={() =>
                    setSelected((old) => old.filter((k) => k !== key))
                  }
                />
                Unavailable channel {key}
              </label>
            ))}
        </fieldset>
        {selected.length > 0 && channelPolicy === "nobody" && (
          <p role="alert">
            Choose an add policy that permits the requested channels.
          </p>
        )}
      </fieldset>
      {catalogChanged && (
        <p role="alert">
          The catalog changed. Reopen Add agent to review the new catalog.
        </p>
      )}
      {!current.fresh && !catalogChanged && (
        <p>Check the host to confirm its catalog before creating.</p>
      )}
      {error && <p role="alert">{error}</p>}
      <div className="flex gap-2">
        <Button size="sm" type="submit" disabled={busy || (!valid && !frozen)}>
          {busy ? "Submitting…" : frozen ? "Retry same request" : "Create"}
        </Button>
        <Button
          size="sm"
          variant="outline"
          type="button"
          disabled={busy}
          onClick={close}
        >
          Close
        </Button>
      </div>
    </form>
  );
}
