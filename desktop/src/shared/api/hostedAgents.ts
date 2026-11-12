import {
  invokeTauri,
  fromRawAcpRuntimeCatalogEntry,
  type RawAcpRuntimeCatalogEntry,
} from "./tauri";
import type {
  AcpRuntimeCatalogEntry,
  AgentModelsResponse,
  CreateManagedAgentInput,
  RegisteredHostDestination,
} from "./types";

export type HostScope = RegisteredHostDestination["scope"];
export type HostedObservation = {
  scope: HostScope;
  agent: string;
  runtime_id: string;
  catalog_revision: string;
  operation_id: string;
  revision: number;
  action_id: string;
  expires_at: number;
  readiness?:
    | "logged-in"
    | "logged-out"
    | "not-applicable"
    | "unknown"
    | "config-invalid"
    | "unsupported";
  availability?: string;
  login: "not-observed" | "in-progress" | "succeeded" | "cancelled" | "failed";
  models:
    | { state: "available"; catalog: AgentModelsResponse }
    | { state: "empty" | "unsupported" }
    | { state: "unavailable"; reason: string };
};
export type HostedSetup = {
  action?: "deploy" | "cleanup";
  scope: HostScope;
  operation: string;
  runtime: string;
  pubkey: string;
  personaId: string | null;
  name: string;
  phase: number;
  model: string | null;
  state: string;
  unit: string | null;
  revision: number | null;
  observation: HostedObservation | null;
  error: string | null;
};
export type HostedCatalog = {
  status: {
    scope: HostScope;
    availability: string;
    expires_at: number;
    capability: { state: string; catalog_revision?: string };
  };
  runtimes: { installedId: string; metadata: AcpRuntimeCatalogEntry | null }[];
};
export type HostedAction =
  | { type: "check" | "retry" | "refresh" | "cleanup" }
  | { type: "launch"; model: string };
export type AgentConfigurationSource =
  | { type: "local" }
  | { type: "hosted"; key: string; scope: HostScope; catalogRevision?: string };
export const LOCAL_CONFIGURATION: AgentConfigurationSource = { type: "local" };
export const hostedSetupToken = () => invokeTauri<string>("hosted_setup_token");
export async function hostedCatalog(scope: HostScope): Promise<HostedCatalog> {
  const raw = await invokeTauri<
    Omit<HostedCatalog, "runtimes"> & {
      runtimes: {
        installedId: string;
        metadata: RawAcpRuntimeCatalogEntry | null;
      }[];
    }
  >("hosted_catalog", { scope });
  return {
    ...raw,
    runtimes: raw.runtimes.map((runtime) => ({
      ...runtime,
      metadata:
        runtime.metadata && fromRawAcpRuntimeCatalogEntry(runtime.metadata),
    })),
  };
}
/** Independent discovery has no setup token or identity side effects. */
export async function hostedModels(input: {
  scope: HostScope;
  runtime: string;
  catalogRevision: string;
}): Promise<AgentModelsResponse> {
  const result = await invokeTauri<
    Pick<HostedObservation, "models" | "readiness" | "availability">
  >("hosted_models", input);
  if (
    result.availability !== "available" ||
    !["logged-in", "not-applicable"].includes(result.readiness ?? "unknown")
  )
    throw new Error(
      "Host harness is not ready. Sign in on the host and retry discovery.",
    );
  if (result.models.state === "available") return result.models.catalog;
  if (result.models.state === "empty")
    return {
      agentName: "Host harness",
      agentVersion: "",
      models: [],
      supportsSwitching: false,
      agentDefaultModel: null,
      selectedModel: null,
    };
  throw new Error(
    `Host models unavailable (${result.models.state === "unavailable" ? result.models.reason : "unsupported"}).`,
  );
}
export const listHostedSetups = (
  expectedRelay: string,
  expectedOwner: string,
) =>
  invokeTauri<HostedSetup[]>("hosted_setup_list", {
    expectedRelay,
    expectedOwner,
  });
export const prepareHostedAgent = (
  input: CreateManagedAgentInput,
  scope: HostScope,
  operation: string,
  runtime: string,
) =>
  invokeTauri<HostedSetup>("hosted_prepare", {
    input,
    scope,
    operation,
    runtime,
  });
export const actOnHostedSetup = (setup: HostedSetup, action: HostedAction) =>
  invokeTauri<HostedSetup>("hosted_setup_action", {
    scope: setup.scope,
    operation: setup.operation,
    runtime: setup.runtime,
    action,
  });

export function currentHostedModels(
  setup: HostedSetup,
  now = Date.now(),
): AgentModelsResponse | null {
  const o = setup.observation;
  if (
    !o ||
    setup.state !== "needs-login" ||
    o.expires_at * 1000 <= now ||
    o.operation_id !== setup.operation ||
    o.agent !== setup.pubkey ||
    o.runtime_id !== setup.runtime ||
    o.revision !== setup.revision ||
    !sameHostScope(o.scope, setup.scope) ||
    o.availability !== "available" ||
    !["logged-in", "not-applicable"].includes(o.readiness ?? "unknown") ||
    o.models.state !== "available"
  )
    return null;
  return o.models.catalog;
}
export function currentHostedCatalog(
  catalog: HostedCatalog | null,
  scope: HostScope,
  now = Date.now(),
): boolean {
  return (
    !!catalog &&
    sameHostScope(catalog.status.scope, scope) &&
    catalog.status.availability === "current" &&
    catalog.status.capability.state === "supported" &&
    catalog.status.expires_at * 1000 > now
  );
}

export function sameHostScope(a: HostScope, b: HostScope): boolean {
  return (
    a.community === b.community &&
    a.owner === b.owner &&
    a.host === b.host &&
    a.generation === b.generation
  );
}
