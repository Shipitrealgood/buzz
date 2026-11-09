import * as React from "react";
import { useCommunities } from "@/features/communities/useCommunities";
import { useIdentityQuery } from "@/shared/api/hooks";
import { invokeTauri } from "@/shared/api/tauri";
import { isWindowsPlatform } from "@/shared/lib/platform";
import { canonicalNpub, truncateNpub } from "@/shared/lib/pubkey";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";

import { useHostDirectory } from "../lib/hostDirectory";
import type { Host, Session } from "../lib/hostDirectory";
export type { Host, Session, HostRequest } from "../lib/hostDirectory";

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
  const mounted = React.useRef(true);
  React.useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  return (
    <div className="space-y-2">
      <Button
        size="sm"
        variant="outline"
        aria-expanded={open}
        disabled={busy}
        onClick={() => {
          setOpen(!open);
          setError("");
        }}
      >
        Add Host
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
              if (mounted.current) setOpen(false);
            } catch (cause) {
              if (mounted.current) setError(message(cause));
            } finally {
              if (mounted.current) {
                setBusy(false);
                reload();
              }
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
            {busy ? "Registering…" : "Submit invitation"}
          </Button>
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => {
              if (input.current) input.current.value = "";
              setOpen(false);
              setError("");
            }}
          >
            Cancel
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
  session,
  reload,
}: {
  item: Host;
  session: Session;
  reload: () => void;
}) {
  const [busy, setBusy] = React.useState(false);
  const [notice, setNotice] = React.useState("");
  const [confirmUnpair, setConfirmUnpair] = React.useState(false);
  const mounted = React.useRef(true);
  React.useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  const statuses: Record<string, string> = {
    "pair-pending": "Registration pending · awaiting host acknowledgement",
    "pair-expired": "Invitation expired · add the host with a new invitation",
    "unpair-pending": "Revocation pending · awaiting host acknowledgement",
    unpaired: "Revoked",
    paired: "Registered",
  };
  async function act(command: "paired_host_refresh" | "paired_host_unpair") {
    setBusy(true);
    setNotice("");
    setConfirmUnpair(false);
    try {
      await invokeTauri<Host>(command, {
        ...session,
        host: item.scope.host,
        generation: item.scope.generation,
      });
    } catch (cause) {
      if (mounted.current) setNotice(message(cause));
    } finally {
      if (mounted.current) {
        setBusy(false);
        reload();
      }
    }
  }
  return (
    <li className="space-y-2 rounded-md border p-3">
      <div className="font-medium">{item.label ?? "Linux host"}</div>
      <details className="text-xs text-muted-foreground">
        <summary className="cursor-pointer">
          Host key · {truncateNpub(item.scope.host)}
        </summary>
        <code className="break-all">
          {canonicalNpub(item.scope.host) ?? item.scope.host}
        </code>
      </details>
      <p className="text-sm">
        {statuses[item.state] ?? "Registration status unknown"}
      </p>
      {item.state === "paired" && (
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => void act("paired_host_refresh")}
          >
            Refresh registration
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => setConfirmUnpair(true)}
          >
            Revoke registration
          </Button>
        </div>
      )}
      {confirmUnpair && (
        <div className="space-y-2">
          <p className="text-sm">
            Revoke this host’s registration for this community?
          </p>
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="destructive"
              disabled={busy}
              onClick={() => void act("paired_host_unpair")}
            >
              Confirm revoke
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
      {notice && (
        <p role="alert" className="text-sm text-destructive">
          {notice}
        </p>
      )}
    </li>
  );
}

function HostDirectory({ session }: { session: Session }) {
  const directory = useHostDirectory(session);
  const reload = () => {
    void directory.refetch();
  };
  const supported =
    directory.data?.supported && directory.data.available && !directory.error;
  return (
    <section aria-label="My hosts" className="space-y-3">
      <h2 className="text-sm font-semibold tracking-tight text-foreground">
        My hosts
      </h2>
      {directory.isPending && (
        <p className="text-sm">Checking host registration support…</p>
      )}
      {directory.data?.available && !directory.data.supported && (
        <p className="text-sm">
          Host registration is not supported by this community.
        </p>
      )}
      {directory.error && (
        <p role="alert" className="text-sm">
          {message(directory.error)}
        </p>
      )}
      {!directory.isPending &&
        (!directory.data?.available || directory.error) && (
          <div className="flex items-center gap-2">
            <p className="text-sm text-muted-foreground">
              Community unavailable. Saved registration information may be out
              of date.
            </p>
            <Button size="sm" variant="outline" onClick={reload}>
              Retry
            </Button>
          </div>
        )}
      {supported && <PairForm session={session} reload={reload} />}
      {supported && !directory.data?.hosts.length && (
        <p className="text-sm text-muted-foreground">
          No registered hosts. Add a Linux host using its private invitation.
        </p>
      )}
      {Boolean(directory.data?.hosts.length) && (
        <p className="text-xs text-muted-foreground">
          Registration records ownership. Connectivity and agent deployment are
          not available here.
        </p>
      )}
      <ul className="grid gap-2">
        {directory.data?.hosts.map((item) => (
          <HostRow
            key={item.scope.generation + item.scope.host}
            item={item}
            session={session}
            reload={reload}
          />
        ))}
      </ul>
    </section>
  );
}

export function MyHosts({ session }: { session: Session }) {
  return (
    <HostDirectory
      key={session.expectedRelay + session.expectedOwner}
      session={session}
    />
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
