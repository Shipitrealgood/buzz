import * as React from "react";

import {
  consumePendingOpenEditAgent,
  type EditAgentFocusTarget,
  subscribeOpenEditAgent,
} from "@/features/agents/openEditAgentEvent";

export function useProfileEditAgentRequest(
  pubkey: string | null | undefined,
  onHostedEdit?: () => void,
) {
  const [open, setOpen] = React.useState(false);
  const [focus, setFocus] = React.useState<EditAgentFocusTarget | undefined>(
    undefined,
  );

  React.useEffect(() => {
    if (!pubkey) {
      return;
    }

    function openRequestedEditor(nextFocus?: EditAgentFocusTarget) {
      if (!nextFocus && onHostedEdit) {
        onHostedEdit();
        return;
      }
      setFocus(nextFocus);
      setOpen(true);
    }
    const pending = consumePendingOpenEditAgent(pubkey);
    if (pending !== false) {
      openRequestedEditor(pending === true ? undefined : pending);
    }

    return subscribeOpenEditAgent(pubkey, openRequestedEditor);
  }, [pubkey, onHostedEdit]);

  return { focus, open, setFocus, setOpen };
}
