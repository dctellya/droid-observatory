//! The frozen v1 rule table: which raw shape of a Droid 0.247 session file becomes which PRD §7
//! kind. The decision file `D-0NN-mapping-v1.md` reproduces it; change both together and bump
//! the mapping version.
//!
//! | Raw shape | Kind | Lane / role |
//! |---|---|---|
//! | `session_start`, no `callingSessionId` | `IntentCreated` | 1, session role |
//! | `session_start` with `callingSessionId` | `SubagentSpawned` | 3, child's role |
//! | root user text typed by the person (`userMessageSource: cli_tui`) | `IntentAmended` | 1, session role |
//! | child user text that is a `Task` or runner assignment (first only) | `TaskStarted` | 3 |
//! | assistant `tool_use` `Task` | `TaskAssigned`, plus `FeatureDefined` the first time a feature is named | 2 |
//! | assistant `tool_use` `ProposeMission` | `PlanCreated`, `MilestoneDefined` per `\| **Mn** ...` row | 2 |
//! | assistant `tool_use` `AskUser` | `QuestionAsked` | 1 |
//! | `tool_result` of an `AskUser` | `IntentAnswered` | 1, session role |
//! | `Execute`/`Script` command with `git commit` | `CommitCreated` | 3 |
//! | `Execute`/`Script` command with `cargo test`, `make test` or `make check` | `TestsRun` | 3 |
//! | `Execute`/`Script` command with `make check` | `GateCommandRun`; its result `GateCommandPassed` / `GateCommandFailed` | 6 |
//! | assistant `tool_use` `Edit`, `Create`, `ApplyPatch` | `FileTouched` | 3 |
//! | `tool_result` with `is_error` | `ToolError`; `HookBlocked` when the text names a hook | 3 |
//! | assistant `modelId` differs from the session's previous one | `ModelChanged` | 3 |
//! | `compaction_state` | `ContextCompacted` | 2 |
//! | `todo_state` whose todos differ from the previous one | `PlanRevised` | 2 |
//! | `agent_turn_outcome` in a child: completed / cancelled / error or usage exhausted | `TaskDone` / `TaskSkipped` / `ToolError` | 3 / 2 / 3 |
//! | `agent_turn_outcome` in the root: cancelled / error or usage exhausted | `MissionPaused` / `EscalationRaised`; completed is quiet | 2 |
//! | child end of file | `SubagentFinished` | 3 |
//! | sidecar `tokenUsage`, at end of file | `UsageReported` | 3 |
//! | scrutiny validator report text | `FindingBlocking`, `FindingNonBlocking`, `ScrutinyPassed` | 4 |
//! | user-testing validator report text | `QaScenarioPassed`, `QaScenarioFailed`, `QaEvidence` | 5 |
//! | thinking, plain text, empty user messages, other tool calls and their results | no event | none |
//! | any other `type`, content part type, tool name or `agent_turn_outcome` reason | `Unknown`, counted | role's lane |
//!
//! An event's `role` is always the role of its session (the capture header's role is the oracle
//! for it); a person's words are told apart by the kind, not by a `Human` role.
//!
//! A shape the table lists as "no event" is understood and has no PRD kind; it is not dropped
//! silently, because the table names every such shape. Anything the table does not name is
//! `Unknown` (rule 18).

/// What a tool call means to the mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tool {
    Task,
    ProposeMission,
    AskUser,
    /// `Execute` (field `command`) and `Script` (field `script`).
    Command,
    Edit,
    /// A tool whose use has no PRD kind.
    Quiet,
}

/// Tools of the 0.247 toolset that carry no PRD kind. `tests`, `discoveredIssues` and
/// `skillFeedback` are not tools: they are keys of an `EndFeatureRun` call whose JSON arguments
/// failed to parse (the M0 capture holds one each).
const QUIET_TOOLS: &[&str] = &[
    "Read",
    "Grep",
    "Glob",
    "LS",
    "TodoWrite",
    "FetchUrl",
    "Skill",
    "WaitForScript",
    "StartMissionRun",
    "EndFeatureRun",
    "DismissHandoffItems",
    "TaskOutput",
    "ToolSearch",
    "store_agent_readiness_report",
    "tests",
    "discoveredIssues",
    "skillFeedback",
];

const EDIT_TOOLS: &[&str] = &["Edit", "Create", "ApplyPatch"];
const COMMAND_TOOLS: &[&str] = &["Execute", "Script"];

/// `None` for a tool the table does not name.
pub(super) fn tool(name: &str) -> Option<Tool> {
    match name {
        "Task" => Some(Tool::Task),
        "ProposeMission" => Some(Tool::ProposeMission),
        "AskUser" => Some(Tool::AskUser),
        name if COMMAND_TOOLS.contains(&name) => Some(Tool::Command),
        name if EDIT_TOOLS.contains(&name) => Some(Tool::Edit),
        name if QUIET_TOOLS.contains(&name) => Some(Tool::Quiet),
        _ => None,
    }
}

const COMMIT_COMMANDS: &[&str] = &["git commit"];
const TEST_COMMANDS: &[&str] = &["cargo test", "make test", "make check"];
const GATE_COMMANDS: &[&str] = &["make check"];

/// What a command line (or the script that runs it) does, by substring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CommandKinds {
    pub commit: bool,
    pub tests: bool,
    pub gate: bool,
}

pub(super) fn classify_command(text: &str) -> CommandKinds {
    let has = |needles: &[&str]| needles.iter().any(|needle| text.contains(needle));
    CommandKinds {
        commit: has(COMMIT_COMMANDS),
        tests: has(TEST_COMMANDS),
        gate: has(GATE_COMMANDS),
    }
}

/// How a turn ended, from `agent_turn_outcome.reason`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Completed,
    Cancelled,
    Failed,
}

pub(super) fn outcome(reason: &str) -> Option<Outcome> {
    match reason {
        "completed" => Some(Outcome::Completed),
        "cancelled" => Some(Outcome::Cancelled),
        "error" | "model_usage_exhausted" => Some(Outcome::Failed),
        _ => None,
    }
}

/// Text in a failed tool result that shows a hook refused the call.
const HOOK_MARKERS: &[&str] = &["guard-git.sh", ".factory/hooks"];

pub(super) fn names_a_hook(text: &str) -> bool {
    HOOK_MARKERS.iter().any(|marker| text.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_are_classified_and_unseen_names_are_not() {
        assert_eq!(tool("Task"), Some(Tool::Task));
        assert_eq!(tool("Script"), Some(Tool::Command));
        assert_eq!(tool("ApplyPatch"), Some(Tool::Edit));
        assert_eq!(tool("Grep"), Some(Tool::Quiet));
        assert_eq!(tool("TeleportFile"), None);
    }

    #[test]
    fn commands_are_classified_by_whole_command_words() {
        let kinds = classify_command("git commit -m x && make check");
        assert_eq!(
            kinds,
            CommandKinds {
                commit: true,
                tests: true,
                gate: true
            }
        );
        assert_eq!(classify_command("ls"), CommandKinds::default());
        for text in [
            "make check",
            "make check;",
            "make check>log",
            "make check 2>&1",
            "make check|tee log",
            "make check&",
            "time make check",
            "A=1 B=2 make check -j4",
            "(cd x && make check)",
            "ls\nmake check\n",
        ] {
            assert!(classify_command(text).gate, "{text:?}");
        }
        for text in [
            "make check-todos",
            "make checks",
            "xmake check",
            "echo make check",
            "echo 'a; make check'",
            "grep \"make check\" Makefile",
            "git commit-tree x",
            "cat <<EOF\nmake check\nEOF",
            "cat <<-'EOF'\n\tmake check\n\tEOF\nls",
            "make",
            "make -C x check",
        ] {
            assert!(!classify_command(text).gate, "{text:?}");
        }
    }

    #[test]
    fn a_heredoc_does_not_hide_the_command_after_it() {
        let kinds = classify_command("git commit -F - <<EOF\nmake check\nEOF\nmake test");
        assert!(kinds.commit && kinds.tests);
        assert!(!kinds.gate);
    }

    #[test]
    fn outcomes_and_hook_markers() {
        assert_eq!(outcome("completed"), Some(Outcome::Completed));
        assert_eq!(outcome("model_usage_exhausted"), Some(Outcome::Failed));
        assert_eq!(outcome("teleported"), None);
        assert!(names_a_hook("blocked by .factory/hooks/guard-git.sh"));
        assert!(!names_a_hook("no such file"));
    }
}
