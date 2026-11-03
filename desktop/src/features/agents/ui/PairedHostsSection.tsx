import { useQuery } from "@tanstack/react-query";
import * as React from "react";
import { useCommunities } from "@/features/communities/useCommunities";
import { useIdentityQuery } from "@/shared/api/hooks";
import { invokeTauri } from "@/shared/api/tauri";
import { isWindowsPlatform } from "@/shared/lib/platform";
import { canonicalNpub, truncateNpub } from "@/shared/lib/pubkey";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";

import { PairedAgentCreate } from "./PairedAgentCreate";
import { PairedHostRequests, terminalHostRequest } from "./PairedHostRequests";

export type HostRequest = {
  request_id: string;
  name: string;
  runtime: string;
  state: string;
  agent: string | null;
  steps: string[];
  issue: string | null;
  evidence_incomplete: boolean;
};
export type Session = { expectedRelay: string; expectedOwner: string };
export type Host = {
  scope: { community: string; generation: string; owner: string; host: string };
  state: string;
  label: string | null;
  runtimes: string[];
  requests: HostRequest[];
  fresh: boolean;
  inventory_id: string | null;
  fresh_for_ms: number;
  catalog_revision: string | null;
  capacity: number | null;
};
type Directory = { supported: boolean; available: boolean; hosts: Host[] };

function message(error: unknown) {
  return error instanceof Error
    ? error.message
    : "The host operation could not finish.";
}

function PairForm({
  session,
  reload,
}: {
  session: Session;
  reload: () => void;
}) {
  const input = React.useRef<HTMLInputElement>(null);
  const [open, setOpen] = React.useState(false);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState("");
  return (
    <div className="space-y-2">
      <Button
        size="sm"
        variant="outline"
        aria-expanded={open}
        onClick={() => {
          setOpen(!open);
          setError("");
        }}
      >
        Pair host
      </Button>
      {open && (
        <form
          className="max-w-md space-y-2"
          onSubmit={async (event) => {
            event.preventDefault();
            if (!input.current || busy) return;
            const invitation = input.current.value.trim();
            input.current.value = "";
            setBusy(true);
            setError("");
            try {
              await invokeTauri<Host>("paired_host_pair", {
                ...session,
                invitation,
              });
              setOpen(false);
            } catch (cause) {
              setError(message(cause));
            } finally {
              setBusy(false);
              reload();
            }
          }}
        >
          <label
            className="text-sm font-medium"
            htmlFor="paired-host-invitation"
          >
            Private host invitation
          </label>
          <Input
            id="paired-host-invitation"
            ref={input}
            type="password"
            required
            maxLength={4096}
            autoComplete="off"
            spellCheck={false}
          />
          <p className="text-xs text-muted-foreground">
            Paste the invitation from the host console. It expires after ten
            minutes.
          </p>
          <Button type="submit" size="sm" disabled={busy}>
            {busy ? "Pairing…" : "Submit invitation"}
          </Button>
        </form>
      )}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}

function HostRow({
  item,
  creating,
  close,
  session,
  reload,
}: {
  item: Host;
  creating: boolean;
  close: () => void;
  session: Session;
  reload: () => void;
}) {
  const [busy, setBusy] = React.useState("");
  const [notice, setNotice] = React.useState("");
  const [confirmed, setConfirmed] = React.useState<Host | null>(null);
  const confirmedUntil = React.useRef(0);
  const activeAction = React.useRef(0);
  const checked = React.useRef(false);
  const [confirmUnpair, setConfirmUnpair] = React.useState(false);
  React.useEffect(() => {
    if (!confirmed) return;
    const timer = window.setTimeout(
      () => setConfirmed(null),
      Math.max(0, confirmedUntil.current - performance.now()),
    );
    return () => window.clearTimeout(timer);
  }, [confirmed]);
  const current =
    item.state === "paired" &&
    confirmed &&
    item.inventory_id === confirmed.inventory_id &&
    performance.now() < confirmedUntil.current
      ? confirmed
      : item;
  const { generation, host } = item.scope;
  const statuses: Record<string, string> = {
    "pair-pending": "Awaiting host acknowledgement",
    "pair-expired": "Invitation expired",
    "unpair-pending": "Unpair pending at host",
    unpaired: "Unpaired",
    paired: current.fresh ? "Host confirmed" : "Paired · inventory cached",
  };
  const act = React.useCallback(
    async (command: "paired_host_refresh" | "paired_host_unpair") => {
      setBusy(command);
      setNotice("");
      setConfirmUnpair(false);
      setConfirmed(null);
      const action = ++activeAction.current;
      const started = performance.now();
      try {
        const result = await invokeTauri<Host>(command, {
          ...session,
          host,
          generation,
        });
        if (activeAction.current !== action) return;
        if (result.fresh && result.inventory_id && result.fresh_for_ms > 0) {
          confirmedUntil.current = started + result.fresh_for_ms;
          setConfirmed(result);
        }
      } catch (cause) {
        if (activeAction.current === action) setNotice(message(cause));
      } finally {
        if (activeAction.current === action) {
          setBusy("");
          reload();
        }
      }
    },
    [session, host, generation, reload],
  );
  React.useEffect(() => {
    if (item.state === "paired" && !item.label && !checked.current) {
      checked.current = true;
      void act("paired_host_refresh");
    }
  }, [item.state, item.label, act]);
  const checkedForCreate = React.useRef(false);
  React.useEffect(() => {
    if (!creating) {
      checkedForCreate.current = false;
      return;
    }
    if (!checkedForCreate.current) {
      checkedForCreate.current = true;
      if (!current.fresh && !busy) void act("paired_host_refresh");
    }
  }, [creating, current.fresh, busy, act]);
  const [reviewed, setReviewed] = React.useState(false);
  React.useEffect(() => {
    if (!creating) setReviewed(false);
    else if (current.fresh) setReviewed(true);
  }, [creating, current.fresh]);
  return (
    <li className="space-y-2 rounded-md border p-3">
      <div className="font-medium">{current.label ?? "Enrolled host"}</div>
      <details className="text-xs text-muted-foreground">
        <summary className="cursor-pointer">
          Host key · {truncateNpub(host)}
        </summary>
        <code className="break-all">{canonicalNpub(host) ?? host}</code>
      </details>
      <p className="text-sm">{statuses[current.state] ?? "Status unknown"}</p>
      {current.runtimes.length > 0 && (
        <p className="text-xs text-muted-foreground">
          Installed runtimes: {current.runtimes.join(", ")}
          {current.capacity ? ` · Capacity: ${current.capacity}` : ""}
        </p>
      )}
      {item.state === "paired" && (
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="outline"
            disabled={Boolean(busy)}
            onClick={() => void act("paired_host_refresh")}
          >
            {busy === "paired_host_refresh" ? "Checking host…" : "Check host"}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy === "paired_host_unpair"}
            onClick={() => setConfirmUnpair(true)}
          >
            Unpair
          </Button>
        </div>
      )}
      {confirmUnpair && (
        <div className="space-y-2">
          <p className="text-sm">
            Unpair this host for this community? Agents already created keep
            running.
          </p>
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="destructive"
              onClick={() => void act("paired_host_unpair")}
            >
              Confirm unpair
            </Button>
            <Button
              size="sm"
              variant="outline"
              onClick={() => setConfirmUnpair(false)}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}
      {creating &&
        item.state === "paired" &&
        (current.fresh || reviewed ? (
          <PairedAgentCreate
            current={current}
            session={session}
            reload={reload}
            close={close}
          />
        ) : (
          <div>
            <p>Confirming installed runtimes…</p>
            <Button size="sm" variant="outline" onClick={close}>
              Cancel create
            </Button>
          </div>
        ))}
      <PairedHostRequests item={item} session={session} reload={reload} />
      {notice && (
        <p role="alert" className="text-sm text-destructive">
          {notice}
        </p>
      )}
    </li>
  );
}

export function MyHosts({ session }: { session: Session }) {
  const [selection, setSelection] = React.useState("");
  const [creating, setCreating] = React.useState("");
  const directory = useQuery({
    queryKey: ["paired-hosts", session.expectedRelay, session.expectedOwner],
    queryFn: () => invokeTauri<Directory>("paired_host_list", session),
    refetchOnWindowFocus: true,
    retry: false,
    staleTime: 15_000,
    refetchInterval: ({ state }) =>
      state.data?.hosts.some(
        (h) =>
          h.state === "pair-pending" ||
          h.state === "unpair-pending" ||
          h.requests?.some(
            (r) =>
              !terminalHostRequest(r.state) &&
              r.state !== "expired-awaiting-host",
          ),
      )
        ? 5000
        : false,
  });
  if (
    directory.isPending ||
    (directory.data?.available && !directory.data.supported)
  )
    return null;
  const reload = () => {
    void directory.refetch();
  };
  return (
    <section aria-label="My hosts" className="space-y-3">
      <h2 className="text-sm font-semibold tracking-tight text-foreground">
        My hosts
      </h2>
      {directory.error && (
        <p role="alert" className="text-sm">
          {message(directory.error)}
        </p>
      )}
      {!directory.data?.available && (
        <div className="flex items-center gap-2">
          <p className="text-sm text-muted-foreground">
            Community unavailable. Saved host information may be out of date.
          </p>
          <Button size="sm" variant="outline" onClick={reload}>
            Retry
          </Button>
        </div>
      )}
      {directory.data?.supported && directory.data.available && (
        <PairForm session={session} reload={reload} />
      )}
      {directory.data?.available && !directory.data.hosts.length && (
        <p className="text-sm text-muted-foreground">
          Pair a Linux host to see its installed runtimes.
        </p>
      )}
      {directory.data?.supported &&
        directory.data.available &&
        directory.data.hosts.some((h) => h.state === "paired") && (
          <div className="flex flex-wrap items-center gap-2">
            <label className="text-sm" htmlFor="paired-create-host">
              Host for new agent
            </label>
            <select
              id="paired-create-host"
              value={selection}
              onChange={(e) => {
                setSelection(e.target.value);
                setCreating("");
              }}
            >
              <option value="">Select a host</option>
              {directory.data.hosts
                .filter((h) => h.state === "paired")
                .map((h) => (
                  <option
                    key={h.scope.generation + h.scope.host}
                    value={h.scope.generation + h.scope.host}
                  >
                    {h.label ?? "Host"} · {truncateNpub(h.scope.host)}
                  </option>
                ))}
            </select>
            <Button
              size="sm"
              disabled={
                !directory.data.hosts.some(
                  (h) =>
                    h.state === "paired" &&
                    h.scope.generation + h.scope.host === selection,
                )
              }
              onClick={() => setCreating(selection)}
            >
              Add agent
            </Button>
          </div>
        )}
      <ul className="grid gap-2">
        {directory.data?.hosts.map((item) => (
          <HostRow
            key={item.scope.generation + item.scope.host}
            creating={creating === item.scope.generation + item.scope.host}
            close={() => setCreating("")}
            item={item}
            session={session}
            reload={reload}
          />
        ))}
      </ul>
    </section>
  );
}

function WindowsHosts() {
  const relay = useCommunities().activeCommunity?.relayUrl;
  const owner = useIdentityQuery().data?.pubkey;
  return relay && owner ? (
    <MyHosts
      key={relay + owner}
      session={{ expectedRelay: relay, expectedOwner: owner }}
    />
  ) : null;
}
export function PairedHostsSection() {
  return isWindowsPlatform() ? <WindowsHosts /> : null;
}
