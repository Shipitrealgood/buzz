import { useIdentityQuery } from "@/shared/api/hooks";
import type { RelayAgent } from "@/shared/api/types";
import {
  canonicalNpub,
  normalizePubkey,
  truncateNpub,
} from "@/shared/lib/pubkey";
import { useIsArchivedPredicate } from "@/features/identity-archive/hooks";
import { useAgentAvailabilityLookup } from "@/features/agents/lib/useAgentAvailability";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import {
  ProfileAvatarWithStatus,
  DEFAULT_HOVER_PROFILE_STATUS_GEOMETRY,
  scaleProfileAvatarStatusGeometry,
} from "@/features/profile/ui/ProfileAvatarWithStatus";
import { AgentIdentityCard } from "./AgentIdentityCard";
import { IDENTITY_CARD_GRID_CLASS } from "./UnifiedAgentsSection";

const avatarSize = 96;
const geometry = scaleProfileAvatarStatusGeometry(
  DEFAULT_HOVER_PROFILE_STATUS_GEOMETRY,
  avatarSize,
);

export function HostAgentsSection({
  agents,
  managedPubkeys,
  onOpenProfile,
}: {
  agents?: RelayAgent[];
  managedPubkeys: ReadonlySet<string>;
  onOpenProfile?: ((pubkey: string) => void) | null;
}) {
  const viewer = useIdentityQuery().data?.pubkey;
  const isArchived = useIsArchivedPredicate();
  const owned = (agents ?? []).filter(
    (agent): agent is RelayAgent & { hostPubkey: string } =>
      Boolean(
        viewer &&
          agent.ownerPubkey === viewer &&
          agent.hostPubkey &&
          !isArchived(agent.pubkey) &&
          !managedPubkeys.has(normalizePubkey(agent.pubkey)),
      ),
  );
  const keys = owned.map((agent) => agent.pubkey);
  const profiles = useUsersBatchQuery(keys, { enabled: keys.length > 0 }).data
    ?.profiles;
  const { getAvailability } = useAgentAvailabilityLookup(keys);
  if (!owned.length) return null;

  return (
    <section
      aria-label="On host"
      className="space-y-3"
      data-testid="host-agents-section"
    >
      <h2 className="text-sm font-semibold tracking-tight text-foreground">
        On host
      </h2>
      <div className={IDENTITY_CARD_GRID_CLASS}>
        {owned.map((agent) => {
          const profile = profiles?.[agent.pubkey];
          const label =
            profile?.displayName?.trim() ||
            agent.name.trim() ||
            truncateNpub(agent.pubkey);
          const avatarUrl = profile?.avatarUrl ?? null;
          const host = truncateNpub(agent.hostPubkey);
          return (
            <AgentIdentityCard
              key={agent.pubkey}
              ariaLabel={`${label}, on ${host}`}
              avatar={
                <ProfileAvatarWithStatus
                  avatarUrl={avatarUrl}
                  label={label}
                  size={avatarSize}
                  geometry={geometry}
                  status={getAvailability(agent.pubkey)}
                  statusTestId={`host-agent-presence-${agent.pubkey}`}
                />
              }
              avatarUrl={avatarUrl}
              dataTestId={`host-agent-${agent.pubkey}`}
              label={label}
              onClick={() => onOpenProfile?.(agent.pubkey)}
              statusBadge={
                <span
                  title={canonicalNpub(agent.hostPubkey) ?? undefined}
                  className="text-xs text-muted-foreground"
                >
                  On {host}
                </span>
              }
            />
          );
        })}
      </div>
    </section>
  );
}
