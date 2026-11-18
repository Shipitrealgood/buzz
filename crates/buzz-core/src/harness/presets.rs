/// Static data for a well-known tier-2 ACP harness.
pub struct PresetHarness {
    /// Canonical main id.
    pub id: &'static str,
    /// Human-readable harness label.
    pub label: &'static str,
    /// Main command.
    pub command: &'static str,
    /// Canonical launch arguments.
    pub args: &'static [&'static str],
    /// Main install instructions url.
    pub install_instructions_url: &'static str,
    /// Main installation guidance.
    pub install_hint: &'static str,
    /// Vendor CLI the ACP command wraps, when the preset is an adapter.
    pub underlying_cli: Option<&'static str>,
    /// State-specific setup guidance for the wrapped vendor CLI.
    pub underlying_cli_install_hint: Option<&'static str>,
    /// Main underlying cli install instructions url.
    pub underlying_cli_install_instructions_url: Option<&'static str>,
}

/// Main compiled preset definitions.
pub const PRESET_HARNESSES: &[PresetHarness] = &[
    PresetHarness {
        id: "pi",
        label: "Pi",
        command: "buzz-pi-acp",
        args: &[],
        install_instructions_url: "https://github.com/salman1993/buzz-pi-acp",
        install_hint: "Requires Node.js 22 or newer. Install the Pi ACP adapter with `npm install -g --install-links=true 'git+https://github.com/salman1993/buzz-pi-acp.git#86b201e'`. Make sure `buzz-pi-acp` is on PATH, then restart Buzz.",
        underlying_cli: Some("pi"),
        underlying_cli_install_hint: Some(
            "Install Pi with `npm install -g @earendil-works/pi-coding-agent`, then run `pi` to configure its model provider.",
        ),
        underlying_cli_install_instructions_url: Some(
            "https://github.com/badlogic/pi-mono/tree/main/packages/coding-agent",
        ),
    },
    PresetHarness {
        id: "devin",
        label: "Devin",
        command: "devin",
        args: &["acp"],
        install_instructions_url: "https://docs.devin.ai/cli",
        install_hint: "Buzz talks to Devin through the official Devin CLI's ACP mode (devin acp).",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "cursor",
        label: "Cursor",
        command: "cursor-agent",
        args: &["acp"],
        install_instructions_url: "https://cursor.com/downloads",
        install_hint: "Buzz talks to Cursor through the cursor-agent CLI's ACP mode.",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "omp",
        label: "Oh My Pi",
        command: "omp",
        args: &["acp"],
        install_instructions_url: "https://omp.sh/",
        install_hint: "Buzz talks to Oh My Pi through its CLI's ACP mode (omp acp).",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "grok",
        label: "Grok Build",
        command: "grok",
        args: &["agent", "--always-approve", "stdio"],
        install_instructions_url: "https://docs.x.ai/build/overview",
        install_hint: "Buzz talks to Grok Build through its CLI's agent stdio mode.",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "opencode",
        label: "OpenCode",
        command: "opencode",
        args: &["acp"],
        install_instructions_url: "https://opencode.ai/docs",
        install_hint: "Buzz talks to OpenCode through its CLI's ACP mode (opencode acp).",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "kimi",
        label: "Kimi Code",
        command: "kimi",
        args: &["acp"],
        install_instructions_url: "https://kimi.ai/download",
        install_hint: "Buzz talks to Kimi Code through its CLI's ACP mode (kimi acp).",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "amp",
        label: "Amp",
        command: "amp-acp",
        args: &[],
        install_instructions_url: "https://github.com/tao12345666333/amp-acp",
        install_hint: "Buzz talks to the Amp CLI through the amp-acp adapter. Follow the setup guide to install the adapter so the amp-acp command is on your PATH.",
        underlying_cli: Some("amp"),
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "hermes",
        label: "Hermes Agent",
        command: "hermes-acp",
        args: &[],
        install_instructions_url: "https://hermes-agent.nousresearch.com",
        install_hint: "Buzz talks to Hermes Agent through its hermes-acp command.",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
    PresetHarness {
        id: "openclaw",
        label: "OpenClaw",
        command: "openclaw",
        args: &["acp"],
        install_instructions_url: "https://docs.openclaw.ai/start/getting-started",
        install_hint: "Buzz talks to OpenClaw through its ACP mode (openclaw acp), which relies on the OpenClaw Gateway daemon. Follow the setup guide to install both.\n\n\
            ⚠️  Execution-locus note: `openclaw acp` runs tools inside the \
            OpenClaw Gateway daemon, not in the Desktop process. \
            Desktop-injected BUZZ_* env vars are visible to the `openclaw` \
            harness process itself, but do NOT automatically reach the \
            Gateway's execution environment. If your tools or agent logic \
            needs BUZZ_* credentials at execution time, set them on the \
            Gateway's own environment separately.",
        underlying_cli: None,
        underlying_cli_install_hint: None,
        underlying_cli_install_instructions_url: None,
    },
];
