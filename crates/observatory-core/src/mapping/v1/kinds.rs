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
//! | `Execute` command that runs `git commit` (whole command words) | `CommitCreated` | 3 |
//! | `Execute` command that runs `cargo test`, `make test` or `make check` | `TestsRun` | 3 |
//! | `Execute` command that runs `make check` | `GateCommandRun`; its result `GateCommandPassed` / `GateCommandFailed` by the exit status in the result text, `is_error` as a second failure signal | 6 |
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
//! | thinking, plain text, empty user messages, `Script` (its inner `Execute` calls are their own records), other tool calls and their results | no event | none |
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
    "Script",
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

/// `None` for a tool the table does not name.
pub(super) fn tool(name: &str) -> Option<Tool> {
    match name {
        "Task" => Some(Tool::Task),
        "ProposeMission" => Some(Tool::ProposeMission),
        "AskUser" => Some(Tool::AskUser),
        "Execute" => Some(Tool::Command),
        name if EDIT_TOOLS.contains(&name) => Some(Tool::Edit),
        name if QUIET_TOOLS.contains(&name) => Some(Tool::Quiet),
        _ => None,
    }
}

const COMMIT_COMMANDS: &[&str] = &["git commit"];
const TEST_COMMANDS: &[&str] = &["cargo test", "make test", "make check"];
const GATE_COMMANDS: &[&str] = &["make check"];

/// What an `Execute` command line runs, by whole command words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct CommandKinds {
    pub commit: bool,
    pub tests: bool,
    pub gate: bool,
}

pub(super) fn classify_command(text: &str) -> CommandKinds {
    let commands = simple_commands(text);
    let runs = |phrases: &[&str]| {
        commands.iter().any(|words| {
            let words = &words[words.iter().take_while(|word| is_prefix_word(word)).count()..];
            phrases.iter().any(|phrase| {
                let wanted: Vec<&str> = phrase.split(' ').collect();
                words.len() >= wanted.len() && words.iter().zip(&wanted).all(|(a, b)| a == b)
            })
        })
    };
    CommandKinds {
        commit: runs(COMMIT_COMMANDS),
        tests: runs(TEST_COMMANDS),
        gate: runs(GATE_COMMANDS),
    }
}

/// Words that come before the program without being it: `time`, `{`, `!` and `NAME=value`.
fn is_prefix_word(word: &str) -> bool {
    matches!(word, "time" | "{" | "!")
        || word.split_once('=').is_some_and(|(name, _)| {
            !name.is_empty()
                && !name.starts_with(|c: char| c.is_ascii_digit())
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// The words of each simple command in a shell line. Quoted text stays one word, heredoc bodies
/// and comments are dropped, and `; & | ( )` and newlines end a command, so a phrase in an
/// `echo "make check"` argument or a commit-message heredoc is never a command.
fn simple_commands(text: &str) -> Vec<Vec<String>> {
    let mut lexer = Lexer::default();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => lexer.end_word(),
            '\n' => {
                lexer.end_command();
                lexer.skip_heredocs(&mut chars);
            }
            ';' | '&' | '|' | '(' | ')' => lexer.end_command(),
            '>' => lexer.end_word(),
            '<' if chars.peek() == Some(&'<') => {
                chars.next();
                if chars.peek() == Some(&'<') {
                    chars.next();
                    lexer.end_word();
                } else {
                    lexer.end_word();
                    lexer.read_heredoc_delimiter(&mut chars);
                }
            }
            '<' => lexer.end_word(),
            '#' if !lexer.in_word => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        lexer.end_command();
                        lexer.skip_heredocs(&mut chars);
                        break;
                    }
                }
            }
            '\'' => {
                lexer.in_word = true;
                chars
                    .by_ref()
                    .take_while(|c| *c != '\'')
                    .for_each(|c| lexer.word.push(c));
            }
            '"' => {
                lexer.in_word = true;
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => lexer.word.extend(chars.next()),
                        c => lexer.word.push(c),
                    }
                }
            }
            '\\' => {
                lexer.in_word = true;
                lexer.word.extend(chars.next());
            }
            c => {
                lexer.in_word = true;
                lexer.word.push(c);
            }
        }
    }
    lexer.end_command();
    lexer.commands
}

#[derive(Default)]
struct Lexer {
    commands: Vec<Vec<String>>,
    words: Vec<String>,
    word: String,
    in_word: bool,
    /// Heredoc delimiters read on the current line: the word and whether `<<-` strips tabs.
    heredocs: Vec<(String, bool)>,
}

impl Lexer {
    fn end_word(&mut self) {
        if self.in_word {
            self.words.push(std::mem::take(&mut self.word));
            self.in_word = false;
        }
    }

    fn end_command(&mut self) {
        self.end_word();
        if !self.words.is_empty() {
            self.commands.push(std::mem::take(&mut self.words));
        }
    }

    fn read_heredoc_delimiter(&mut self, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
        let strip_tabs = chars.next_if_eq(&'-').is_some();
        while chars.next_if(|c| matches!(c, ' ' | '\t')).is_some() {}
        let mut delimiter = String::new();
        let mut quote = None;
        while let Some(&c) = chars.peek() {
            match (c, quote) {
                ('\'' | '"', None) => quote = Some(c),
                (c, Some(open)) if c == open => quote = None,
                (' ' | '\t' | '\n' | ';' | '&' | '|' | '(' | ')' | '<' | '>', None) => break,
                (c, _) => delimiter.push(c),
            }
            chars.next();
        }
        self.heredocs.push((delimiter, strip_tabs));
    }

    /// Called right after a newline: drops the body lines of every heredoc opened on the line
    /// before it.
    fn skip_heredocs(&mut self, chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
        for (delimiter, strip_tabs) in std::mem::take(&mut self.heredocs) {
            loop {
                let line: String = chars.by_ref().take_while(|c| *c != '\n').collect();
                let line = if strip_tabs {
                    line.trim_start_matches('\t')
                } else {
                    &line
                };
                if line == delimiter || chars.peek().is_none() {
                    break;
                }
            }
        }
    }
}

/// The exit status of an `Execute` result, from its text: Droid 0.247 sets `is_error` on neither
/// a failing nor a passing command and writes the status only into the text, as a closing
/// `[Process exited with code N]` line or an opening `Command failed (exit code: N)`.
pub(super) fn exit_code(text: &str) -> Option<i32> {
    let closing = text
        .trim_end()
        .strip_suffix(']')
        .and_then(|rest| rest.rsplit_once("[Process exited with code "))
        .and_then(|(_, code)| code.parse().ok());
    closing.or_else(|| {
        let opening = text.strip_prefix("Error: ").unwrap_or(text);
        opening
            .strip_prefix("Command failed (exit code: ")?
            .split_once(')')?
            .0
            .parse()
            .ok()
    })
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
        assert_eq!(tool("Execute"), Some(Tool::Command));
        assert_eq!(tool("Script"), Some(Tool::Quiet));
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
    fn the_exit_status_is_read_from_the_result_text() {
        let failed = "Command failed (exit code: 2)\n\n[Process exited with code 2]";
        assert_eq!(exit_code(failed), Some(2));
        assert_eq!(
            exit_code("Error: Command failed (exit code: 130)"),
            Some(130)
        );
        assert_eq!(exit_code("ok\n\n\n[Process exited with code 0]\n"), Some(0));
        assert_eq!(
            exit_code("log says exit code: 2\n[Process exited with code 0]"),
            Some(0)
        );
        assert_eq!(exit_code("see Command failed (exit code: 2) above"), None);
        assert_eq!(exit_code("all green"), None);
        assert_eq!(exit_code(""), None);
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
