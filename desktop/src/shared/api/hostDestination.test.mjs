import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
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

test("Windows-only hosted exports have matching command registration gates", () => {
  const agents = readFileSync(
    new URL("../../../src-tauri/src/commands/agents.rs", import.meta.url),
    "utf8",
  );
  const app = readFileSync(
    new URL("../../../src-tauri/src/lib.rs", import.meta.url),
    "utf8",
  );
  const exports = agents.match(
    /#\[cfg\(windows\)\]\s*pub use hosted::\{([^}]+)\};/,
  );
  assert.ok(exports, "hosted command exports must be Windows-only");
  const commands = exports[1]
    .split(",")
    .map((name) => name.trim())
    .filter(Boolean);
  assert.ok(commands.includes("hosted_models"));
  const handlers = app.slice(app.indexOf("tauri::generate_handler!["));
  for (const command of commands) {
    assert.equal(handlers.match(new RegExp(`\\b${command},`, "g"))?.length, 1);
    assert.match(
      handlers,
      new RegExp(`#\\[cfg\\(windows\\)\\]\\s*${command},`),
      command,
    );
  }
});
