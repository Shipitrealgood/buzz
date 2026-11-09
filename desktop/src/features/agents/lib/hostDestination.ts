import type { Host, Session } from "./hostDirectory";
import type { RegisteredHostDestination } from "@/shared/api/types";

/** UI keys live in a namespace separate from external provider IDs. */
export function hostDestinationValue(host: Host): string {
  return `host:${host.scope.community}:${host.scope.owner}:${host.scope.host}:${host.scope.generation}`;
}

/** Defense against a late or incorrectly scoped directory projection. */
export function hostBelongsToSession(host: Host, session: Session): boolean {
  try {
    const relay = new URL(session.expectedRelay);
    relay.protocol = relay.protocol === "wss:" ? "https:" : relay.protocol;
    return (
      host.scope.owner === session.expectedOwner &&
      host.scope.community === relay.origin
    );
  } catch {
    return false;
  }
}

/** Registration-only truth: freshness is not online status or deploy support. */
export function hostRegistrationStatus(host: Host): string {
  switch (host.state) {
    case "unpaired":
      return "Revoked";
    case "unpair-pending":
      return "Revocation pending";
    case "pair-pending":
      return "Registration pending";
    case "pair-expired":
      return "Registration expired";
    case "paired":
      return host.fresh
        ? "Registered · deployment unavailable"
        : "Saved registration · refresh needed";
    default:
      return "Registration status unknown";
  }
}

export function hostDestination(host: Host): RegisteredHostDestination {
  return { type: "registered-host", scope: { ...host.scope } };
}
