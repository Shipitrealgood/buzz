import { expect, test, type Page } from "@playwright/test";
import { installMockBridge } from "../helpers/bridge";
import { waitForAnimations } from "../helpers/animations";
import type { HostedSetup } from "../../src/shared/api/hostedAgents";

type Fixture = {
  calls: { command: string; args?: Record<string, unknown> }[];
  setup: HostedSetup | null;
  denied: boolean;
  login: boolean;
  expire: boolean;
  lose: boolean;
  registered: boolean;
  models: boolean;
  missingModel: boolean;
  delayNext: boolean;
  release?: () => void;
  hostedStart?: number;
};
declare global {
  interface Window {
    __HOSTED_SETUP_FIXTURE__: Fixture;
  }
}
async function open(page: Page, channel = false) {
  await page.addInitScript(() =>
    Object.defineProperty(navigator, "platform", {
      value: "Win32",
      configurable: true,
    }),
  );
  await installMockBridge(
    page,
    {
      addChannelMembersErrors: ["Fixture membership denied", null],
      managedAgents: [
        {
          pubkey: "70".repeat(32),
          name: "Synthetic record template",
          status: "stopped",
        },
      ],
    },
    { relayWsUrl: "wss://community.example" },
  );
  await page.goto("/");
  await page.getByTestId("open-agents-view").waitFor();
  await page.evaluate(() => {
    const api = (
      window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (
            command: string,
            args?: Record<string, unknown>,
          ) => Promise<unknown>;
        };
      }
    ).__TAURI_INTERNALS__;
    const previous = api.invoke.bind(api);
    const scope = {
      community: "https://community.example",
      owner: "deadbeef".repeat(8),
      host: "71".repeat(32),
      generation: "73".repeat(32),
    };
    const fixture: Fixture = {
      calls: [],
      setup: null,
      denied: false,
      login: false,
      expire: false,
      lose: false,
      registered: true,
      models: true,
      missingModel: false,
      delayNext: false,
    };
    window.__HOSTED_SETUP_FIXTURE__ = fixture;
    let record: Record<string, unknown> | null = null;
    const observed = (): HostedSetup["observation"] =>
      fixture.setup && {
        scope,
        agent: fixture.setup.pubkey,
        runtime_id: fixture.setup.runtime,
        catalog_revision: "74".repeat(32),
        operation_id: fixture.setup.operation,
        revision: fixture.setup.revision ?? 1,
        action_id: "75".repeat(32),
        expires_at: Math.floor(Date.now() / 1000) + (fixture.expire ? -1 : 120),
        login: fixture.login ? "succeeded" : "not-observed",
        models:
          fixture.login && fixture.models
            ? {
                state: "available",
                catalog: {
                  agentName: "Synthetic Grok",
                  agentVersion: "fixture",
                  models: fixture.missingModel
                    ? []
                    : [
                        {
                          id: "opaque/model:fixture",
                          name: "Synthetic model",
                          description: null,
                        },
                      ],
                  agentDefaultModel: null,
                  selectedModel: null,
                  supportsSwitching: true,
                },
              }
            : { state: "unavailable", reason: "network-disabled" },
      };
    api.invoke = async (command, args) => {
      fixture.calls.push({ command, args: structuredClone(args) });
      if (command === "paired_host_list")
        return {
          supported: true,
          available: true,
          hosts: [
            {
              scope,
              state: fixture.registered ? "paired" : "unpaired",
              label: "Synthetic workshop",
              requests: [],
              runtimes: [],
              fresh: true,
              fresh_for_ms: 60000,
              inventory_id: null,
              catalog_revision: null,
              capacity: null,
            },
          ],
        };
      if (command === "hosted_setup_token") return "76".repeat(32);
      if (command === "hosted_setup_list")
        return fixture.setup
          ? [
              {
                ...structuredClone(fixture.setup),
                observation: null,
                state: "unverified",
              },
            ]
          : [];
      if (command === "hosted_catalog") {
        if (!fixture.registered || fixture.denied)
          throw new Error("Host metadata read denied");
        const catalog = {
          status: {
            scope,
            availability: "current",
            expires_at: Math.floor(Date.now() / 1000) + 120,
            capability: {
              state: "supported",
              catalog_revision: "74".repeat(32),
            },
          },
          runtimes: [
            {
              installedId: "native/grok",
              metadata: {
                id: "grok",
                label: "Grok Build",
                avatar_url: "",
                availability: "available",
                command: "grok",
                binary_path: null,
                default_args: ["agent", "--always-approve", "stdio"],
                mcp_command: null,
                model_env_var: null,
                provider_env_var: null,
                thinking_env_var: null,
                max_tokens_env_var: null,
                context_limit_env_var: null,
                max_rounds_env_var: null,
                install_hint: "",
                install_instructions_url: "",
                can_auto_install: false,
                requires_external_cli: false,
                underlying_cli_path: null,
                node_required: false,
                auth_status: { status: "not_applicable" },
                login_hint: null,
                source: "preset",
                definition_env: {},
              },
            },
          ],
        };
        catalog.runtimes.push({
          ...catalog.runtimes[0],
          installedId: "second/grok",
        });
        return catalog;
      }
      if (command === "list_managed_agents") {
        const records = (await previous(command, args)) as Record<
          string,
          unknown
        >[];
        return record ? [...records, record] : records;
      }
      if (command === "hosted_prepare") {
        const input = args?.input as { name: string; personaId: string };
        if (!fixture.setup) {
          const template = (
            (await previous("list_managed_agents")) as Record<string, unknown>[]
          )[0];
          record = {
            ...template,
            pubkey: "72".repeat(32),
            name: input.name,
            persona_id: input.personaId,
            backend: { type: "registered_host", scope },
            status: "host_pending",
            model: null,
            agent_command: "grok",
            start_on_app_launch: false,
          };
          fixture.setup = {
            scope,
            operation: String(args?.operation),
            runtime: String(args?.runtime),
            pubkey: String(record.pubkey),
            personaId: input.personaId,
            name: input.name,
            phase: 3,
            model: null,
            state: "needs-login",
            unit: "77".repeat(32),
            revision: 1,
            observation: null,
            error: null,
          };
          fixture.setup.observation = observed();
        }
        if (fixture.lose) {
          fixture.lose = false;
          throw new Error("Fixture lost prepare response");
        }
        return structuredClone(fixture.setup);
      }
      if (command === "hosted_setup_action") {
        if (fixture.denied || !fixture.registered)
          throw new Error("Current deployment read denied");
        if (!fixture.setup) throw new Error("No retained fixture setup");
        const action = args?.action as { type: string; model?: string };
        if (action.type === "refresh") {
          fixture.setup.operation = "78".repeat(32);
          fixture.setup.revision = 2;
          fixture.setup.state = "needs-login";
          fixture.setup.observation = observed();
        }
        if (action.type === "launch") {
          fixture.setup.operation = "79".repeat(32);
          fixture.setup.revision = 3;
          fixture.setup.model = action.model ?? null;
          fixture.setup.state = "failed";
          fixture.setup.observation = null;
          fixture.setup.error =
            "Host networking is disabled; deployment was not applied.";
        }
        if (action.type === "check" && fixture.setup.state === "needs-login")
          fixture.setup.observation = observed();
        const response = structuredClone(fixture.setup);
        if (fixture.delayNext) {
          fixture.delayNext = false;
          await new Promise<void>((resolve) => {
            fixture.release = resolve;
          });
        }
        return response;
      }
      return previous(command, args);
    };
  });
  if (channel) {
    await page.getByTestId("channel-random").click();
    await page.getByTestId("channel-intro-action-create-agent").click();
    await page.getByTestId("add-channel-create-agent").click();
  } else {
    await page.getByTestId("open-agents-view").click();
    await page.getByTestId("new-agent-card").click();
  }
  await page.locator("#persona-display-name").fill("Hosted synthetic agent");
  await page.getByRole("button", { name: "Advanced", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Add Host", exact: true }).last(),
  ).toBeVisible();
  await page.locator("#agent-run-on").press("Enter");
  await page.getByRole("menuitemradio", { name: /Synthetic workshop/ }).click();
  await expect(
    page.getByRole("button", { name: "Agent harness", exact: true }),
  ).toContainText("Grok Build · native/grok");
  await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    f.hostedStart = f.calls.length;
  });
  await page
    .getByRole("button", { name: "Agent harness", exact: true })
    .click();
  await expect(
    page.getByRole("menuitemradio", { name: /Grok Build · second\/grok/ }),
  ).toBeVisible();
  await page
    .getByRole("menuitemradio", { name: /Grok Build · native\/grok/ })
    .click();
}

test("ordinary hosted form retains one setup through close, native login, explicit model refresh and offline failure", async ({
  page,
}) => {
  await open(page);
  await expect(
    page.getByRole("button", { name: "Prepare agent", exact: true }),
  ).toBeEnabled();
  // Two same-tick form submits exercise the real shared latch before definition effects.
  await page.locator("#persona-dialog-form").evaluate((form) => {
    form.dispatchEvent(
      new Event("submit", { bubbles: true, cancelable: true }),
    );
    form.dispatchEvent(
      new Event("submit", { bubbles: true, cancelable: true }),
    );
  });
  const setup = page.getByTestId("hosted-agent-setup");
  await expect(setup).toContainText("Needs login");
  await expect(setup).toContainText("Networking is disabled");
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeDisabled();
  const firstHostedCalls = await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    return f.calls.slice(f.hostedStart);
  });
  expect(
    firstHostedCalls.filter((c) => c.command === "discover_agent_models"),
  ).toEqual([]);
  await setup.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByTestId("new-agent-card").click();
  await page.getByRole("button", { name: "Advanced", exact: true }).click();
  await page
    .getByRole("button", {
      name: "Continue setup: Hosted synthetic agent",
      exact: true,
    })
    .click();
  await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    f.hostedStart = f.calls.length;
  });
  await setup
    .getByRole("button", { name: "Check status", exact: true })
    .click();
  await page.evaluate(() => {
    window.__HOSTED_SETUP_FIXTURE__.login = true;
  });
  // Native login alone does not update renderer models.
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeDisabled();
  await setup
    .getByRole("button", { name: "Refresh models", exact: true })
    .click();
  await setup.locator("#persona-model").click();
  await page
    .getByRole("button", { name: "Synthetic model", exact: true })
    .click();
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeEnabled();
  await expect(setup).toContainText("Runs on Synthetic workshop");
  await expect(setup).not.toContainText("buzz-host-activate");
  await expect(setup).not.toContainText("Complete native login");
  await waitForAnimations(page);
  await setup.screenshot({ path: "test-results/hosted-setup-models.png" });
  await setup.getByRole("button", { name: "Launch", exact: true }).click();
  await expect(setup).toContainText("Host networking is disabled");
  await setup
    .getByRole("button", { name: "Retry request", exact: true })
    .click();
  const calls = await page.evaluate(
    () => window.__HOSTED_SETUP_FIXTURE__.calls,
  );
  expect(calls.filter((c) => c.command === "create_persona")).toHaveLength(1);
  expect(calls.filter((c) => c.command === "hosted_prepare")).toHaveLength(1);
  expect(
    calls.filter((c) =>
      /^(create_managed_agent|start_managed_agent|deploy_to_provider|login_acp_runtime)$/.test(
        c.command,
      ),
    ),
  ).toEqual([]);
  const hostedCalls = calls.slice(
    await page.evaluate(() => window.__HOSTED_SETUP_FIXTURE__.hostedStart ?? 0),
  );
  expect(
    hostedCalls.filter((c) =>
      /^(discover_agent_models|login_acp_runtime|get_acp_commands|load_runtime_file_config)$/.test(
        c.command,
      ),
    ),
  ).toEqual([]);
  const selected = calls.filter((c) => c.command === "hosted_setup_action");
  expect(selected.at(-1)?.args?.operation).toBe("79".repeat(32));
});

test("retained setup denies launch after expired or denied current evidence", async ({
  page,
}) => {
  await open(page);
  await page
    .getByRole("button", { name: "Prepare agent", exact: true })
    .click();
  const setup = page.getByTestId("hosted-agent-setup");
  await expect(setup).toBeVisible();
  await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    f.login = true;
    f.models = false;
  });
  await setup
    .getByRole("button", { name: "Refresh models", exact: true })
    .click();
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeDisabled();
  await expect(setup).toContainText("Native login: succeeded");
  await expect(setup).toContainText("Networking is disabled");
  await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    f.models = true;
  });
  await setup
    .getByRole("button", { name: "Check status", exact: true })
    .click();
  await setup.locator("#persona-model").click();
  await page
    .getByRole("button", { name: "Synthetic model", exact: true })
    .click();
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeEnabled();
  await page.evaluate(() => {
    window.__HOSTED_SETUP_FIXTURE__.missingModel = true;
  });
  await setup
    .getByRole("button", { name: "Check status", exact: true })
    .click();
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeDisabled();
  await expect(setup).toContainText(
    "Selected model: opaque/model:fixture (unavailable)",
  );
  await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    f.models = true;
    f.missingModel = false;
    f.expire = true;
  });
  await setup
    .getByRole("button", { name: "Check status", exact: true })
    .click();
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeDisabled();
  await page.evaluate(() => {
    window.__HOSTED_SETUP_FIXTURE__.denied = true;
  });
  await setup
    .getByRole("button", { name: "Check status", exact: true })
    .click();
  await expect(setup.getByRole("alert")).toContainText("read denied");
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeDisabled();
});

test("contextual hosted preparation keeps its identity through lost response and channel retry", async ({
  page,
}) => {
  await open(page, true);
  await page.evaluate(() => {
    window.__HOSTED_SETUP_FIXTURE__.lose = true;
  });
  await page
    .getByRole("button", { name: "Prepare agent", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Prepare agent", exact: true }),
  ).toBeEnabled();
  await page
    .getByRole("button", { name: "Prepare agent", exact: true })
    .click();
  const setup = page.getByTestId("hosted-agent-setup");
  await expect(setup).toBeVisible();
  const toast = page
    .locator("[data-sonner-toast][data-removed='false']")
    .filter({ hasText: "Fixture membership denied" });
  await expect(toast).toBeVisible();
  await setup.getByRole("button", { name: "Close", exact: true }).click();
  await toast.getByRole("button", { name: "Try again" }).click();
  await expect(
    page
      .locator("[data-sonner-toast][data-removed='false']")
      .filter({ hasText: "Added Hosted synthetic agent to #random" }),
  ).toBeVisible();
  const calls = await page.evaluate(
    () => window.__HOSTED_SETUP_FIXTURE__.calls,
  );
  expect(calls.filter((c) => c.command === "create_persona")).toHaveLength(1);
  expect(calls.filter((c) => c.command === "hosted_prepare")).toHaveLength(2);
  const memberships = calls.filter((c) => c.command === "add_channel_members");
  expect(memberships).toHaveLength(2);
  expect(memberships[0].args).toEqual(memberships[1].args);
  expect(JSON.stringify(memberships[0].args)).toContain("72".repeat(32));
  expect(
    calls.filter((c) =>
      /^(start_managed_agent|create_managed_agent|deploy_to_provider)$/.test(
        c.command,
      ),
    ),
  ).toEqual([]);
  await page.getByTestId("open-agents-view").click();
  await page
    .getByRole("button", {
      name: "Hosted synthetic agent agent profile",
      exact: true,
    })
    .click();
  await page.getByRole("tab", { name: "Runtime", exact: true }).click();
  await expect(page.getByTestId("edit-agent-run-on")).toContainText(
    "Runs on Synthetic workshop",
  );
  await expect(
    page.getByTestId("edit-agent-run-on").locator("svg.lucide-cloud"),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Continue setup", exact: true })
    .click();
  await expect(setup).toBeVisible();
  // A late result from a closed setup cannot replace the next mounted setup.
  await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    f.delayNext = true;
    f.login = true;
  });
  await setup
    .getByRole("button", { name: "Check status", exact: true })
    .click();
  await expect
    .poll(() => page.evaluate(() => !!window.__HOSTED_SETUP_FIXTURE__.release))
    .toBe(true);
  await setup.getByRole("button", { name: "Close", exact: true }).click();
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await page.getByTestId("open-agents-view").click();
  await page.getByTestId("new-agent-card").click();
  await page.getByRole("button", { name: "Advanced", exact: true }).click();
  await page
    .getByRole("button", {
      name: "Continue setup: Hosted synthetic agent",
      exact: true,
    })
    .click();
  await page.evaluate(() => {
    const f = window.__HOSTED_SETUP_FIXTURE__;
    f.registered = false;
    f.release?.();
  });
  await setup
    .getByRole("button", { name: "Check status", exact: true })
    .click();
  await expect(setup.getByRole("alert")).toContainText("read denied");
  await expect(
    setup.getByRole("button", { name: "Launch", exact: true }),
  ).toBeDisabled();
});
