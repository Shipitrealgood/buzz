import assert from "node:assert/strict";
import test from "node:test";
import { createManagedAgent } from "./tauri.ts";

test("shared create API refuses host or malformed destination before all native IPC", async () => {
  let calls = 0;
  globalThis.window = {
    __TAURI_INTERNALS__: {
      invoke() {
        calls++;
        throw new Error("unexpected native call");
      },
    },
  };
  for (const destination of [
    { type: "registered-host", scope: {} },
    {},
    false,
    "local",
  ]) {
    await assert.rejects(
      createManagedAgent({
        name: "Fixture",
        destination,
        backend: { type: "local" },
      }),
      /Registered-host creation is not available/,
    );
  }
  assert.equal(calls, 0);
});
