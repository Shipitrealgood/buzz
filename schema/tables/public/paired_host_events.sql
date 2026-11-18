CREATE TABLE paired_host_events (
    community_id UUID NOT NULL,
    event_id BYTEA NOT NULL CHECK (octet_length(event_id) = 32),
    generation BYTEA NOT NULL CHECK (octet_length(generation) = 32),
    owner_key BYTEA NOT NULL CHECK (octet_length(owner_key) = 32),
    host_key BYTEA NOT NULL CHECK (octet_length(host_key) = 32),
    source_bucket BYTEA CHECK (octet_length(source_bucket) = 32),
    operation TEXT NOT NULL,
    request_id BYTEA CHECK (octet_length(request_id) = 32),
    prev_id BYTEA CHECK (octet_length(prev_id) = 32),
    expires_at BIGINT NOT NULL,
    seq BIGINT GENERATED ALWAYS AS IDENTITY,
    -- Preserve the original signed NIP-01 serialization for transcript recovery.
    event_json TEXT NOT NULL,
    route_json JSONB NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (community_id, event_id),
    UNIQUE (community_id, seq)
);
CREATE INDEX idx_paired_host_events_mailbox ON paired_host_events
    (community_id, generation, owner_key, host_key, seq);
CREATE INDEX idx_paired_host_events_offer ON paired_host_events
    (community_id, generation, host_key, operation, seq);
CREATE INDEX idx_paired_host_events_owner ON paired_host_events
    (community_id, owner_key, operation, expires_at);
CREATE INDEX idx_paired_host_events_host ON paired_host_events
    (community_id, host_key, operation, expires_at);
CREATE INDEX idx_paired_host_events_source ON paired_host_events
    (community_id, source_bucket, expires_at) WHERE source_bucket IS NOT NULL;
