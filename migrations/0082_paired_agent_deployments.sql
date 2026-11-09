-- Fork-only expand; migration 81 and registration v2 remain unchanged.
SET LOCAL lock_timeout = '5s';
SET LOCAL statement_timeout = '30s';

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

CREATE TABLE public.paired_agent_operations (
    community_id uuid NOT NULL,
    owner_pubkey bytea NOT NULL,
    operation_id text NOT NULL,
    agent_pubkey bytea NOT NULL,
    host_pubkey bytea NOT NULL,
    generation bytea NOT NULL,
    action text NOT NULL,
    expected_revision bigint NOT NULL,
    expires_at bigint NOT NULL,
    content_sha256 text NOT NULL,
    event_json text NOT NULL,
    state text NOT NULL,
    result_json text,
    permit_nonce text,
    permit_expires_at bigint,
    PRIMARY KEY (community_id, owner_pubkey, operation_id)
);
CREATE INDEX paired_agent_operations_host ON public.paired_agent_operations (community_id, host_pubkey, generation, operation_id);

CREATE TABLE public.paired_host_deployment_status (
    community_id uuid NOT NULL,
    generation bytea NOT NULL,
    owner_pubkey bytea NOT NULL,
    host_pubkey bytea NOT NULL,
    expires_at bigint NOT NULL,
    event_json text NOT NULL,
    PRIMARY KEY (community_id, generation)
);
