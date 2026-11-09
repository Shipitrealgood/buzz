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
