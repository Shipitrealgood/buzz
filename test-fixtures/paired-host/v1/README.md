# Paired-host v1 test vectors

These frozen Rust-produced events use public, disposable test identities.
The owner secret is the 32-byte big-endian scalar 1 and the host secret is
the 32-byte big-endian scalar 2. The generation is byte `ab` repeated
32 times and the console proof is byte `cd` repeated 32 times.
Use Unix second 1800000001 as verification time.

Each file contains the signed NIP-01 `event` and expected decrypted JSON
object `plaintext`. Verify signatures and event IDs, decrypt with the
recipient's key and sender's pubkey, then compare the decoded fields.
The acknowledgement references the exact event ID in `claim.json`.
Random encryption nonces and signatures were frozen together.

After an owner restart, exercise each row of `restore.json` using the original
claim and ack. Treat `expected_claim_id` and the invitation scope as local
state. The result describes receipt verification at the row's Unix second.
An invalid claim ID fails during restore. A historical receipt verifies the
transcript and requires fresh host inventory before use.
