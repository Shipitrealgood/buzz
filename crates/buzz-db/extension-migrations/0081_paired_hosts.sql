-- Private application-owned transport. No generic event-store access.
SET LOCAL lock_timeout = '5s';

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

CREATE TABLE paired_host_bindings (
    community_id UUID NOT NULL,
    generation BYTEA NOT NULL CHECK (octet_length(generation) = 32),
    owner_key BYTEA NOT NULL CHECK (octet_length(owner_key) = 32),
    host_key BYTEA NOT NULL CHECK (octet_length(host_key) = 32),
    claim_id BYTEA NOT NULL CHECK (octet_length(claim_id) = 32),
    ack_id BYTEA NOT NULL CHECK (octet_length(ack_id) = 32),
    state TEXT NOT NULL CHECK (state IN ('active', 'fenced', 'dead')),
    fence_seq BIGINT,
    tombstone_id BYTEA CHECK (octet_length(tombstone_id) = 32),
    retention_floor BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (community_id, generation)
);
CREATE UNIQUE INDEX idx_paired_host_bindings_live ON paired_host_bindings
    (community_id, host_key) WHERE state IN ('active', 'fenced');
CREATE INDEX idx_paired_host_bindings_owner ON paired_host_bindings
    (community_id, owner_key, state, generation);
CREATE INDEX idx_paired_host_bindings_host ON paired_host_bindings
    (community_id, host_key, generation);

CREATE TABLE paired_host_requests (
    community_id UUID NOT NULL,
    generation BYTEA NOT NULL CHECK (octet_length(generation) = 32),
    owner_key BYTEA NOT NULL CHECK (octet_length(owner_key) = 32),
    host_key BYTEA NOT NULL CHECK (octet_length(host_key) = 32),
    request_id BYTEA NOT NULL CHECK (octet_length(request_id) = 32),
    intent_id BYTEA NOT NULL CHECK (octet_length(intent_id) = 32),
    head_phase TEXT NOT NULL,
    generation_challenge_id BYTEA CHECK (octet_length(generation_challenge_id) = 32),
    confirm_id BYTEA CHECK (octet_length(confirm_id) = 32),
    offer_id BYTEA CHECK (octet_length(offer_id) = 32),
    activate_id BYTEA CHECK (octet_length(activate_id) = 32),
    commit_id BYTEA CHECK (octet_length(commit_id) = 32),
    credentials_id BYTEA CHECK (octet_length(credentials_id) = 32),
    terminal_id BYTEA CHECK (octet_length(terminal_id) = 32),
    intent_expiry BIGINT NOT NULL,
    PRIMARY KEY (community_id, generation, owner_key, host_key, request_id),
    UNIQUE (community_id, intent_id)
);
CREATE INDEX idx_paired_host_requests_phase ON paired_host_requests
    (community_id, generation, owner_key, host_key, head_phase);
