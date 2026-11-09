CREATE TABLE public.paired_host_deployment_status (
    community_id uuid NOT NULL,
    generation bytea NOT NULL,
    owner_pubkey bytea NOT NULL,
    host_pubkey bytea NOT NULL,
    expires_at bigint NOT NULL,
    event_json text NOT NULL,
    PRIMARY KEY (community_id, generation)
);
