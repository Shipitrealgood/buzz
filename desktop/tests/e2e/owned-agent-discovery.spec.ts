import { expect, test, type Page } from "@playwright/test";
import { installMockBridge, openNewMessagePage } from "../helpers/bridge";
import { waitForAnimations } from "../helpers/animations";

const AGENT = "a7".repeat(32);
const HOST = "ca".repeat(32);
const FLEET = ["Claude", "Grok", "Scout"].map((name, i) => ({
  name,
  pubkey: ["ab", "ac", "ad"][i].repeat(32),
}));
const OWNER = "deadbeef".repeat(8);

async function setupHostFleet(page: Page) {
  await installMockBridge(page, {
    managedAgents: [],
    searchProfiles: FLEET.map((agent) => ({
      pubkey: agent.pubkey,
      displayName: agent.name,
      ownerPubkey: OWNER,
      isAgent: true,
    })),
    relayAgents: [
      ...FLEET.map((agent) => ({
        ...agent,
        hostPubkey: HOST,
        ownerPubkey: OWNER,
        respondTo:
          agent.name === "Scout"
            ? ("owner-only" as const)
            : ("anyone" as const),
        channelAddPolicy: "owner_only" as const,
        channelNames: ["general"],
      })),
      {
        name: "Foreign host agent",
        pubkey: "ae".repeat(32),
        ownerPubkey: "af".repeat(32),
        hostPubkey: HOST,
        respondTo: "anyone",
        channelAddPolicy: "owner_only",
        channelNames: ["general"],
      },
      {
        pubkey: AGENT,
        channelAddPolicy: "owner_only",
        name: "Policy-only Scout",
        ownerPubkey: OWNER,
        status: "unknown",
        respondTo: "owner-only",
        channelNames: [],
        channelIds: [],
      },
    ],
  });
  await page.goto("/");
}

test("New Message keeps authenticated owner without a user-search duplicate", async ({
  page,
}) => {
  await setupHostFleet(page);
  await openNewMessagePage(page);
  await page.getByTestId("new-dm-search").fill("Policy-only Scout");
  const row = page.getByTestId(`new-dm-result-${AGENT}`);
  await expect(row).toBeVisible();
  await expect(row).toContainText("managed by you");
  await waitForAnimations(page);
  await row.screenshot({
    path: "test-results/owned-agent-discovery/new-message-owner.png",
  });
});

test("member-add keeps authenticated owner without a user-search duplicate", async ({
  page,
}) => {
  await setupHostFleet(page);
  await page.getByTestId("channel-general").click();
  await page.getByTestId("channel-members-trigger").click();
  await page
    .getByTestId("channel-management-search-users")
    .fill("Policy-only Scout");
  const row = page.getByTestId(`channel-user-search-result-${AGENT}`);
  await expect(row).toBeVisible();
  await expect(row).toContainText("managed by you");
  await waitForAnimations(page);
  await row.screenshot({
    path: "test-results/owned-agent-discovery/member-add-owner.png",
  });
});

test("fresh owner sees one read-only card per host agent and can open its conversation", async ({
  page,
}) => {
  await setupHostFleet(page);
  await page.getByTestId("open-agents-view").click();
  const section = page.getByRole("region", { name: "On host" });
  for (const agent of FLEET) {
    const card = section.getByTestId(`host-agent-${agent.pubkey}`);
    await expect(card).toHaveCount(1);
    await expect(card).toContainText(agent.name);
    await expect(card).toContainText("On npub1");
  }
  await expect(section.getByRole("button")).toHaveCount(3);
  await expect(section.getByRole("button", { name: /start/i })).toHaveCount(0);
  await expect(section.getByText("Foreign host agent")).toHaveCount(0);
  await section.getByRole("button", { name: /^Scout,/ }).click();
  await expect(page.getByTestId("user-profile-message")).toBeVisible();
  await page.getByTestId("user-profile-message").click();
  await expect(page.getByTestId("message-input")).toBeVisible();
});

test("member-add hides a foreign owner-only add even when the agent responds to anyone", async ({
  page,
}) => {
  await setupHostFleet(page);
  await page.getByTestId("channel-general").click();
  await page.getByTestId("channel-members-trigger").click();
  const search = page.getByTestId("channel-management-search-users");
  await search.fill("Policy-only Scout");
  await expect(
    page.getByTestId(`channel-user-search-result-${AGENT}`),
  ).toBeVisible();
  await search.fill("Foreign host agent");
  await expect(
    page.getByTestId(`channel-user-search-result-${AGENT}`),
  ).toHaveCount(0);
  await expect(
    page.getByTestId(`channel-user-search-result-${"ae".repeat(32)}`),
  ).toHaveCount(0);
});

test("cold member search checks ambiguous identities and can retry ordinary people independently", async ({
  page,
}) => {
  const human = "b1".repeat(32);
  const bot = "b2".repeat(32);
  await installMockBridge(page, {
    managedAgents: [],
    relayAgentDirectoryUnavailable: true,
    searchProfiles: [
      { pubkey: human, displayName: "Probe Human", isAgent: false },
      { pubkey: bot, displayName: "Probe Bot", isAgent: false },
    ],
    relayAgents: [
      {
        pubkey: bot,
        name: "Probe Bot",
        respondTo: "anyone",
        channelAddPolicy: "nobody",
      },
    ],
  });
  await page.goto("/");
  await page.getByTestId("channel-general").click();
  await page.getByTestId("channel-members-trigger").click();
  await page.evaluate(() => {
    const w = window as unknown as {
      __TAURI_INTERNALS__: {
        invoke: (cmd: string, args?: unknown) => Promise<unknown>;
      };
      failExact?: () => void;
      checked?: boolean;
    };
    const invoke = w.__TAURI_INTERNALS__.invoke;
    w.__TAURI_INTERNALS__.invoke = async (cmd, args) => {
      if (cmd === "check_member_add_people" && !w.checked) {
        w.checked = true;
        return new Promise((_, reject) => {
          w.failExact = () => reject(new Error("simulated lookup failure"));
        });
      }
      return invoke(cmd, args);
    };
  });
  const input = page.getByTestId("channel-management-search-users");
  await input.fill("Probe");
  await expect
    .poll(() =>
      page.evaluate(() =>
        Boolean((window as unknown as { failExact?: unknown }).failExact),
      ),
    )
    .toBe(true);
  const row = (key: string) =>
    page.getByTestId(`channel-user-search-result-${key}`);
  await expect(row(bot)).toHaveCount(0);
  await expect(row(human)).toHaveCount(0);
  await input.press("Enter");
  await page.evaluate(() =>
    (window as unknown as { failExact: () => void }).failExact(),
  );
  await expect(row(bot)).toHaveCount(0);
  await page.getByRole("button", { name: "Retry lookup" }).click();
  await expect(row(human)).toBeVisible();
  await expect(row(bot)).toHaveCount(0);
  await row(human).getByRole("button", { name: "Add", exact: true }).click();
  const writes = () =>
    page.evaluate(() =>
      (
        window as unknown as {
          __BUZZ_E2E_COMMAND_PAYLOADS__: Array<{
            command: string;
            payload: {
              pubkeys?: string[];
              expectedRelayUrl?: string;
              expectedSignerPubkey?: string;
            };
          }>;
        }
      ).__BUZZ_E2E_COMMAND_PAYLOADS__
        .filter((x) => x.command === "add_channel_members")
        .map((x) => x.payload),
    );
  await expect.poll(writes).toEqual([
    expect.objectContaining({
      pubkeys: [human],
      expectedRelayUrl: expect.any(String),
      expectedSignerPubkey: OWNER,
    }),
  ]);
});
