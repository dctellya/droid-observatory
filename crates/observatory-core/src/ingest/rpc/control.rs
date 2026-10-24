use std::collections::HashMap;
use std::io::{self, Write};

use serde::Serialize;
use serde_json::{Value, json};

use super::open::OpenRequests;
use super::protocol::{
    HostControl, OPTION_APPROVE, OPTION_DENY, RpcResponse, encode, encode_initialize,
};
use crate::view::LaunchConfig;

/// What `send` wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sent {
    /// The id of the request written, or of Droid's request that was answered.
    pub id: String,
    /// The method of the request written; `"response"` for an answer to Droid's request.
    pub method: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AckOutcome {
    Accepted,
    Rejected { code: i64, message: String },
}

/// Droid's response to a request the host wrote, matched by id.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ack {
    pub id: String,
    pub method: String,
    pub outcome: AckOutcome,
}

#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    /// The question or permission request is unknown, already answered or already resolved.
    #[error("{id} is not an open request: it is unknown, answered or resolved")]
    NotOpen { id: String },
    #[error("request {request_id} does not offer the option {option}")]
    OptionUnavailable { request_id: String, option: String },
    #[error("cannot write to the hosted process: {0}")]
    Io(#[from] io::Error),
}

/// Writes controls to the hosted process (its stdin) and matches Droid's responses to them.
pub struct ControlChannel<W: Write> {
    writer: W,
    next_id: u64,
    pending: HashMap<String, &'static str>,
}

impl<W: Write> ControlChannel<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            next_id: 1,
            pending: HashMap::new(),
        }
    }

    pub fn writer(&self) -> &W {
        &self.writer
    }

    /// Requests written and not yet acknowledged.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Writes the `droid.initialize_session` request that starts the hosted mission; Droid
    /// answers it with the session id.
    pub fn initialize(
        &mut self,
        config: &LaunchConfig,
        session_id: &str,
    ) -> Result<Sent, ControlError> {
        let request = encode_initialize(config, session_id, self.next_id);
        self.write(&request)?;
        self.next_id += 1;
        self.pending.insert(request.id.clone(), request.method);
        Ok(Sent {
            id: request.id,
            method: request.method,
        })
    }

    /// Writes the control as one line. Pause, resume and message are requests, acknowledged by
    /// a response with the same id (`observe`). Answer, approve and deny are responses to a
    /// request of Droid's still open in `open`; that request is closed once written.
    pub fn send(
        &mut self,
        control: &HostControl,
        open: &mut OpenRequests,
    ) -> Result<Sent, ControlError> {
        if let Some(request) = encode(control, self.next_id) {
            self.write(&request)?;
            self.next_id += 1;
            self.pending.insert(request.id.clone(), request.method);
            return Ok(Sent {
                id: request.id,
                method: request.method,
            });
        }
        let (id, result) = match control {
            HostControl::Answer { question_id, text } => {
                let question =
                    open.question_for_event(question_id)
                        .ok_or_else(|| ControlError::NotOpen {
                            id: question_id.clone(),
                        })?;
                let answers: Vec<Value> = if question.prompts.is_empty() {
                    vec![json!({ "index": 0, "question": "", "answer": text })]
                } else {
                    question
                        .prompts
                        .iter()
                        .map(|prompt| {
                            json!({ "index": prompt.index, "question": prompt.question, "answer": text })
                        })
                        .collect()
                };
                let (id, call) = (question.request_id.clone(), question.tool_call_id.clone());
                open.close_call(&call);
                (id, json!({ "answers": answers }))
            }
            HostControl::Approve { request_id } => (
                permission(open, request_id, OPTION_APPROVE)?,
                selected(OPTION_APPROVE),
            ),
            HostControl::Deny { request_id } => (
                permission(open, request_id, OPTION_DENY)?,
                selected(OPTION_DENY),
            ),
            HostControl::Pause | HostControl::Resume | HostControl::Message { .. } => {
                unreachable!("encode yields a request for these controls")
            }
        };
        self.write(&RpcResponse::new(&id, result))?;
        if let HostControl::Approve { request_id } | HostControl::Deny { request_id } = control {
            open.close_permission(request_id);
        }
        Ok(Sent {
            id,
            method: "response",
        })
    }

    /// Reads one stdout line. A response whose id is a pending request acknowledges it.
    pub fn observe(&mut self, line: &[u8]) -> Option<Ack> {
        let value: Value = serde_json::from_slice(line).ok()?;
        if value.get("method").is_some() {
            return None;
        }
        let id = value.get("id")?.as_str()?;
        let method = self.pending.remove(id)?;
        let outcome = match value.get("error") {
            Some(error) => AckOutcome::Rejected {
                code: error["code"].as_i64().unwrap_or_default(),
                message: error["message"].as_str().unwrap_or_default().to_owned(),
            },
            None => AckOutcome::Accepted,
        };
        Some(Ack {
            id: id.to_owned(),
            method: method.to_owned(),
            outcome,
        })
    }

    fn write(&mut self, line: &impl Serialize) -> Result<(), ControlError> {
        let mut bytes = serde_json::to_vec(line).map_err(io::Error::from)?;
        bytes.push(b'\n');
        self.writer.write_all(&bytes)?;
        self.writer.flush()?;
        Ok(())
    }
}

fn selected(option: &str) -> Value {
    json!({ "selectedOption": option })
}

/// The id of the open permission request, if it offers `option` (or lists no options).
fn permission(open: &OpenRequests, request_id: &str, option: &str) -> Result<String, ControlError> {
    let request = open
        .permission(request_id)
        .ok_or_else(|| ControlError::NotOpen {
            id: request_id.to_owned(),
        })?;
    if !request.options.is_empty() && !request.options.iter().any(|offered| offered == option) {
        return Err(ControlError::OptionUnavailable {
            request_id: request_id.to_owned(),
            option: option.to_owned(),
        });
    }
    Ok(request.request_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::rpc::open::OpenPermission;

    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_failed_write_registers_no_pending_request() {
        let mut channel = ControlChannel::new(Broken);

        let sent = channel.send(&HostControl::Pause, &mut OpenRequests::default());

        assert!(matches!(sent, Err(ControlError::Io(_))), "{sent:?}");
        assert_eq!(channel.pending(), 0);
    }

    #[test]
    fn an_option_the_request_does_not_offer_is_refused_and_leaves_it_open() {
        let mut open = OpenRequests::default();
        open.observe_permission(OpenPermission {
            request_id: "p1".into(),
            options: vec!["cancel".into()],
            confirmation: None,
        });
        let mut channel = ControlChannel::new(Vec::new());

        let approved = channel.send(
            &HostControl::Approve {
                request_id: "p1".into(),
            },
            &mut open,
        );

        assert!(matches!(
            approved,
            Err(ControlError::OptionUnavailable { .. })
        ));
        assert!(channel.writer().is_empty());
        assert!(open.permission("p1").is_some());
    }

    #[test]
    fn lines_that_are_not_responses_acknowledge_nothing() {
        let mut channel = ControlChannel::new(Vec::new());
        channel
            .send(&HostControl::Pause, &mut OpenRequests::default())
            .unwrap();

        assert_eq!(channel.observe(b"not json"), None);
        assert_eq!(
            channel.observe(br#"{"id":"1","method":"droid.ask_user"}"#),
            None
        );
        assert_eq!(channel.observe(br#"{"result":{}}"#), None);
        assert_eq!(channel.pending(), 1);
    }
}
