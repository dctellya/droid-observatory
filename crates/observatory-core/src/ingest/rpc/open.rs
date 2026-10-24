use std::collections::{HashMap, HashSet};

/// One question of a `droid.ask_user` request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub index: u64,
    pub question: String,
}

/// A `droid.ask_user` request that has no answer yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenQuestion {
    /// JSON-RPC id of Droid's request: the answer is a response with this id.
    pub request_id: String,
    /// The `tool_use` id of the `AskUser` call, which is also what the transcript calls it.
    pub tool_call_id: String,
    pub prompts: Vec<Prompt>,
}

/// A `droid.request_permission` request that has no answer yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenPermission {
    pub request_id: String,
    /// The `value` of each option Droid offers; empty when the request lists none.
    pub options: Vec<String>,
    /// `confirmationType` of the request: what is being confirmed.
    pub confirmation: Option<String>,
    /// The names of the tools the request asks to run (`toolUses[].toolUse.name`).
    pub tools: Vec<String>,
}

/// `confirmationType` of the permission request for a mission proposal, when Droid sets one.
const PROPOSE_MISSION: &str = "propose_mission";
/// The tool the orchestrator calls to propose its plan; Droid 0.247.0 names the request by it
/// and sets no `confirmationType`.
const PROPOSE_MISSION_TOOL: &str = "ProposeMission";

impl OpenPermission {
    /// The orchestrator proposes its plan and waits. `droid exec --mission` accepts the
    /// proposal without asking; a hosted session has to answer it.
    pub fn is_mission_proposal(&self) -> bool {
        self.confirmation.as_deref() == Some(PROPOSE_MISSION)
            || self.tools.iter().any(|tool| tool == PROPOSE_MISSION_TOOL)
    }
}

/// A question that waits for an answer, with the id of the `QuestionAsked` event to answer it by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedQuestion {
    pub event_id: String,
    pub prompts: Vec<Prompt>,
}

/// The requests of Droid's that wait for the human. A question is known by the id of its
/// `QuestionAsked` event, which the mapper binds to the tool call; a request that was answered,
/// or whose tool call already has a result, is closed and cannot be answered again.
#[derive(Debug, Default)]
pub struct OpenRequests {
    questions: Vec<OpenQuestion>,
    permissions: Vec<OpenPermission>,
    event_calls: HashMap<String, String>,
    closed_calls: HashSet<String>,
}

impl OpenRequests {
    pub fn questions(&self) -> &[OpenQuestion] {
        &self.questions
    }

    pub fn permissions(&self) -> &[OpenPermission] {
        &self.permissions
    }

    pub fn observe_question(&mut self, question: OpenQuestion) {
        let known = self
            .questions
            .iter()
            .any(|open| open.request_id == question.request_id);
        if !known && !self.closed_calls.contains(&question.tool_call_id) {
            self.questions.push(question);
        }
    }

    pub fn observe_permission(&mut self, permission: OpenPermission) {
        if !self
            .permissions
            .iter()
            .any(|open| open.request_id == permission.request_id)
        {
            self.permissions.push(permission);
        }
    }

    /// Ties the `QuestionAsked` event `event_id` to the `AskUser` call it was mapped from.
    pub fn bind_event(&mut self, event_id: &str, tool_call_id: &str) {
        self.event_calls
            .insert(event_id.to_owned(), tool_call_id.to_owned());
    }

    /// The call has a result: whoever answered, the question is closed.
    pub fn close_call(&mut self, tool_call_id: &str) {
        self.closed_calls.insert(tool_call_id.to_owned());
        self.questions
            .retain(|open| open.tool_call_id != tool_call_id);
    }

    pub fn close_permission(&mut self, request_id: &str) {
        self.permissions
            .retain(|open| open.request_id != request_id);
    }

    /// The open question the `QuestionAsked` event `event_id` asked, if it is still open.
    pub fn question_for_event(&self, event_id: &str) -> Option<&OpenQuestion> {
        let call = self.event_calls.get(event_id)?;
        self.questions
            .iter()
            .find(|open| &open.tool_call_id == call)
    }

    /// The questions still open, in the order they were asked, each by its event id.
    pub fn open_questions(&self) -> Vec<AskedQuestion> {
        self.questions
            .iter()
            .flat_map(|open| {
                self.event_calls
                    .iter()
                    .filter(|(_, call)| **call == open.tool_call_id)
                    .map(|(event, _)| AskedQuestion {
                        event_id: event.clone(),
                        prompts: open.prompts.clone(),
                    })
            })
            .collect()
    }

    pub fn permission(&self, request_id: &str) -> Option<&OpenPermission> {
        self.permissions
            .iter()
            .find(|open| open.request_id == request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn question(request: &str, call: &str) -> OpenQuestion {
        OpenQuestion {
            request_id: request.into(),
            tool_call_id: call.into(),
            prompts: vec![Prompt {
                index: 1,
                question: "q".into(),
            }],
        }
    }

    #[test]
    fn a_closed_call_cannot_be_reopened_by_a_late_request() {
        let mut open = OpenRequests::default();
        open.bind_event("s:1:0", "toolu_1");
        open.close_call("toolu_1");

        open.observe_question(question("r1", "toolu_1"));

        assert!(open.questions().is_empty());
        assert_eq!(open.question_for_event("s:1:0"), None);
    }

    #[test]
    fn a_repeated_request_is_kept_once_and_an_unbound_event_finds_nothing() {
        let mut open = OpenRequests::default();
        open.observe_question(question("r1", "toolu_1"));
        open.observe_question(question("r1", "toolu_1"));
        assert_eq!(open.questions().len(), 1);
        assert_eq!(open.question_for_event("s:1:0"), None);

        open.bind_event("s:1:0", "toolu_1");
        assert_eq!(open.question_for_event("s:1:0").unwrap().request_id, "r1");
    }

    #[test]
    fn permissions_are_found_by_request_id_until_closed() {
        let mut open = OpenRequests::default();
        let permission = OpenPermission {
            request_id: "p1".into(),
            options: vec!["cancel".into()],
            confirmation: None,
            tools: Vec::new(),
        };
        open.observe_permission(permission.clone());
        open.observe_permission(permission.clone());
        assert_eq!(open.permissions().len(), 1);
        assert_eq!(open.permission("p1"), Some(&permission));

        open.close_permission("p1");
        assert_eq!(open.permission("p1"), None);
    }
}
