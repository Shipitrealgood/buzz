import { expect, test } from "@playwright/test";
import { installMockBridge, openNewMessagePage } from "../helpers/bridge";
import { waitForAnimations } from "../helpers/animations";

const AGENT = "a7".repeat(32);
const HOST = "ca".repeat(32);
const FLEET = ["Claude", "Grok", "Scout"].map((name, i) => ({
  name,
  pubkey: ["ab", "ac", "ad"][i].repeat(32),
}));
const OWNER = "deadbeef".repeat(8);

test.beforeEach(async ({ page }) => {
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
});

test("New Message keeps authenticated owner without a user-search duplicate", async ({
  page,
}) => {
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
