CREATE TABLE public.paired_agent_placements (
    community_id uuid NOT NULL,
    agent_pubkey bytea NOT NULL,
    owner_pubkey bytea NOT NULL,
    host_pubkey bytea NOT NULL,
    generation bytea NOT NULL,
    revision bigint NOT NULL,
    unit_id text NOT NULL,
    state text NOT NULL,
    last_operation_id text NOT NULL,
    cleanup_operation_id text,
    PRIMARY KEY (community_id, agent_pubkey)
);
CREATE INDEX paired_agent_placements_host ON public.paired_agent_placements (community_id, host_pubkey, generation);
