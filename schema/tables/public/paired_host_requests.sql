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
