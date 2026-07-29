import type { BackendIntent } from "../lib/instanceInputForDefinition";
import type { BackendProviderProbeResult } from "@/shared/api/types";
import { coerceConfigValues } from "./ProviderConfigFields";

/** Draft state of the optional remote-backend selector. */
export type WhereToRunDraft = {
  runOn: "local" | string;
  providerConfig: Record<string, string>;
  probedProvider: BackendProviderProbeResult | null;
};

export const emptyWhereToRunDraft: WhereToRunDraft = {
  runOn: "local",
  providerConfig: {},
  probedProvider: null,
};

/**
 * Seed schema defaults into a provider config without clobbering typed values.
 *
 * Only fields the user has left empty receive the schema `default`. The
 * previous behavior (replacing the whole config with defaults after every
 * probe) wiped the user's input as they typed, because the probe effect
 * re-ran on each draft change.
 */
export function seedProviderConfigDefaults(
  schema: Record<string, unknown> | undefined,
  current: Record<string, string>,
): Record<string, string> {
  const properties = (schema?.properties ?? {}) as Record<
    string,
    Record<string, unknown>
  >;
  const seeded: Record<string, string> = { ...current };
  for (const [key, property] of Object.entries(properties)) {
    if ((seeded[key] ?? "").trim() === "" && property.default != null) {
      seeded[key] = String(property.default);
    }
  }
  return seeded;
}

export function providerConfigComplete(draft: WhereToRunDraft): boolean {
  if (draft.runOn === "local") return true;
  if (!draft.probedProvider) return false;
  const schema = draft.probedProvider.config_schema as
    | Record<string, unknown>
    | undefined;
  const required: string[] = (schema?.required as string[] | undefined) ?? [];
  return required.every(
    (key) => (draft.providerConfig[key] ?? "").trim().length > 0,
  );
}

export function canSubmitWhereToRun(draft: WhereToRunDraft): boolean {
  return providerConfigComplete(draft);
}

export function resolveBackendIntent(
  draft: WhereToRunDraft,
): BackendIntent | null {
  if (draft.runOn === "local") return null;
  return {
    type: "provider",
    id: draft.runOn,
    config: coerceConfigValues(
      draft.providerConfig,
      draft.probedProvider?.config_schema,
    ),
  };
}
