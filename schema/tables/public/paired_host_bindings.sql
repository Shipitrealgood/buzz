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
