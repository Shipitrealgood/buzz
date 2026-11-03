import * as React from "react";
import { invokeTauri } from "@/shared/api/tauri";
import { Button } from "@/shared/ui/button";
import { truncateNpub } from "@/shared/lib/pubkey";
import type { Host, Session, HostRequest } from "./PairedHostsSection";

export function terminalHostRequest(state: string) {
  return ["ready", "failed", "expired", "revoked-before-commit"].includes(
    state,
  );
}
const labels: Record<string, string> = {
  ready: "Ready",
  failed: "Failed",
  expired: "Expired",
  "expired-awaiting-host": "Expired · awaiting host outcome",
  pending: "Pending",
  "awaiting-commit": "Awaiting host commit",
  committed: "Committed · provisioning",
  "pending-publication": "Confirming community publication",
  verifying: "Verifying public identity and channels",
  "revoked-before-commit": "Revoked before commit",
  unresolved: "Evidence incomplete",
};
export function PairedHostRequests({
  item,
  session,
  reload,
}: {
  item: Host;
  session: Session;
  reload: () => void;
}) {
  const initiated = React.useRef(new Set<string>());
  const [error, setError] = React.useState("");
  const [remote, setRemote] = React.useState<string[]>([]);
  const [cursor, setCursor] = React.useState<string | null>(null);
  const [open, setOpen] = React.useState(false);
  const [observed, setObserved] = React.useState<Record<string, string>>({});
  const [loading, setLoading] = React.useState(false);
  const args = React.useMemo(
    () => ({
      ...session,
      host: item.scope.host,
      generation: item.scope.generation,
    }),
    [session, item.scope.host, item.scope.generation],
  );
  const resume = React.useCallback(
    async (r: string) => {
      setError("");
      try {
        await invokeTauri<HostRequest>("paired_host_resume", {
          ...args,
          requestId: r,
        });
      } catch (e) {
        setError(
          e instanceof Error ? e.message : "Unable to resume this request.",
        );
      }
    },
    [args],
  );
  React.useEffect(() => {
    for (const r of item.requests ?? []) {
      if (
        !terminalHostRequest(r.state) &&
        r.state !== "expired-awaiting-host" &&
        !initiated.current.has(r.request_id)
      ) {
        initiated.current.add(r.request_id);
        void resume(r.request_id);
      }
    }
  }, [item.requests, resume]);
  async function history(after: string | null) {
    setOpen(true);
    setLoading(true);
    setError("");
    try {
      const [ids, next] = await invokeTauri<[string[], string | null]>(
        "paired_host_history_ids",
        { ...args, after },
      );
      setRemote((old) => [...new Set(after ? [...old, ...ids] : ids)]);
      setCursor(next);
    } catch (e) {
      setError(e instanceof Error ? e.message : "History unavailable.");
    } finally {
      setLoading(false);
    }
  }
  async function inspect(r: string) {
    setLoading(true);
    setError("");
    try {
      const v = await invokeTauri<{
        name: string;
        host_state: string;
        agent: string | null;
      }>("paired_host_history", { ...args, requestId: r });
      setObserved((old) => ({
        ...old,
        [r]:
          v.name +
          " · host reported " +
          (labels[v.host_state] ?? v.host_state) +
          (v.agent ? ` · ${truncateNpub(v.agent)}` : ""),
      }));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to verify history.");
    } finally {
      setLoading(false);
    }
  }
  return (
    <div className="space-y-2">
      {(item.requests ?? []).map((r) => (
        <fieldset
          key={r.request_id}
          className="space-y-1 text-sm"
          aria-label={`${r.name} provisioning`}
        >
          <p>
            {r.name} · {r.runtime} ·{" "}
            <strong>{labels[r.state] ?? "Status unknown"}</strong>
          </p>
          {r.agent && <p title={r.agent}>Agent {truncateNpub(r.agent)}</p>}
          {r.steps.length > 0 && (
            <p className="text-xs">{r.steps.join(", ")}</p>
          )}
          {r.evidence_incomplete && (
            <p role="status">
              Evidence incomplete. Waiting for verified request history.
            </p>
          )}
          {r.issue && <p role="status">{r.issue}</p>}
          {!terminalHostRequest(r.state) && (
            <Button
              size="sm"
              variant="outline"
              onClick={async () => {
                await resume(r.request_id);
                reload();
              }}
            >
              Resume request
            </Button>
          )}
        </fieldset>
      ))}
      {item.state !== "pair-pending" && item.state !== "pair-expired" && (
        <Button
          size="sm"
          variant="outline"
          aria-expanded={open}
          disabled={loading}
          onClick={() => (open ? setOpen(false) : void history(null))}
        >
          Request history
        </Button>
      )}
      {open && (
        <section aria-label="Read-only host history">
          {!remote.length && !loading && <p>No retained requests.</p>}
          {remote.map((r) => (
            <div key={r}>
              {observed[r] ?? (
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={loading}
                  onClick={() => void inspect(r)}
                >
                  Inspect request {r.slice(0, 12)}
                </Button>
              )}
            </div>
          ))}
          {cursor && (
            <Button
              size="sm"
              disabled={loading}
              onClick={() => void history(cursor)}
            >
              More requests
            </Button>
          )}
        </section>
      )}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
