import { useQuery } from "@tanstack/react-query";
import { invokeTauri } from "@/shared/api/tauri";

export type HostRequest = {
  request_id: string;
  name: string;
  runtime: string;
  state: string;
  agent: string | null;
  steps: string[];
  issue: string | null;
  evidence_incomplete: boolean;
};
export type Session = { expectedRelay: string; expectedOwner: string };
export type Host = {
  scope: { community: string; generation: string; owner: string; host: string };
  state: string;
  label: string | null;
  runtimes: string[];
  requests: HostRequest[];
  fresh: boolean;
  inventory_id: string | null;
  fresh_for_ms: number;
  catalog_revision: string | null;
  capacity: number | null;
};
export type Directory = {
  supported: boolean;
  available: boolean;
  hosts: Host[];
};

/** Shared private registration query; cache identity includes owner and workspace. */
export function useHostDirectory(session: Session | null) {
  return useQuery({
    queryKey: ["paired-hosts", session?.expectedRelay, session?.expectedOwner],
    queryFn: () => {
      if (!session) throw new Error("Select a community and identity first.");
      return invokeTauri<Directory>("paired_host_list", session);
    },
    enabled: session !== null,
    placeholderData: undefined,
    refetchOnWindowFocus: true,
    retry: false,
    staleTime: 15_000,
    refetchInterval: ({ state }) =>
      state.data?.hosts.some(
        (host) =>
          host.state === "pair-pending" || host.state === "unpair-pending",
      )
        ? 5000
        : 60_000,
  });
}
