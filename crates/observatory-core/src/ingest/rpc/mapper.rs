use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use super::envelope::{Incoming, parse};
use super::open::OpenRequests;
use super::session_file::SessionFile;
use crate::config::IngestConfig;
use crate::ingest::RawRecord;
use crate::mapping::{Mapper, MappingVersion, SessionMeta};
use crate::model::{CycleTemplate, Event, EventKind, SessionId, Timestamp};

/// Maps the lines of a spool through mapping v1, the rules that map a session file: the
/// stream's messages are turned back into the file's records in memory and the events point
/// at the spool line. A line that is already a session record is mapped as it is.
///
/// The session is the one the `droid.initialize_session` response names, so a hosted mission
/// and the same mission attached from its file share session and mission ids. Until that
/// response is read, a line takes the launch id for a session.
pub struct RpcMapper {
    launch: SessionId,
    version: MappingVersion,
    template: Arc<CycleTemplate>,
    mapper: Option<Mapper>,
    open: OpenRequests,
    protocol: BTreeMap<String, u64>,
    files: Option<SessionFile>,
}

impl RpcMapper {
    /// A mapper for the engineering cycle (D-024).
    pub fn new(launch_id: &str, version: MappingVersion) -> Self {
        Self::with_template(launch_id, version, CycleTemplate::engineering())
    }

    /// A mapper that assigns events to the lanes of `template` (F-041).
    pub fn with_template(
        launch_id: &str,
        version: MappingVersion,
        template: impl Into<Arc<CycleTemplate>>,
    ) -> Self {
        Self {
            launch: SessionId(launch_id.to_owned()),
            version,
            template: template.into(),
            mapper: None,
            open: OpenRequests::default(),
            protocol: BTreeMap::new(),
            files: None,
        }
    }

    /// Also reads the hosted child's session file, found below `[ingest].factory_sessions_dir`
    /// by the session id the stream names, for the records the stream has no notification for:
    /// `todo_state`, which mapping v1 turns into `plan.revised`. The events carry the session
    /// file's pointers and so the ids the attach path gives the same lines. A plan the stream
    /// also carries, or a file read twice, maps once: mapping v1 reports a plan only when it
    /// differs from the one before.
    #[must_use]
    pub fn with_session_files(mut self, cfg: &IngestConfig) -> Self {
        self.files = Some(SessionFile::new(cfg));
        self
    }

    /// The session of the stream, once its first response has been read.
    pub fn session_id(&self) -> Option<&SessionId> {
        self.mapper.as_ref().map(|mapper| &mapper.session().id)
    }

    /// Lines that carry state of the process and no lane event, by name. Counted so that no
    /// line goes unaccounted for; a line the rules do not know is an `Unknown` event instead.
    pub fn protocol_counts(&self) -> &BTreeMap<String, u64> {
        &self.protocol
    }

    pub fn open_requests(&self) -> &OpenRequests {
        &self.open
    }

    pub fn open_requests_mut(&mut self) -> &mut OpenRequests {
        &mut self.open
    }

    pub fn map(&mut self, record: &RawRecord) -> Vec<Event> {
        match parse(&record.bytes) {
            Incoming::Record => {
                let own = time_of(&record.bytes);
                self.feed(record, record.bytes.clone(), own)
            }
            Incoming::Session { id, record: line } => {
                self.mapper.get_or_insert_with(|| {
                    Mapper::with_template(
                        SessionMeta::root(SessionId(id)),
                        self.version,
                        self.template.clone(),
                    )
                });
                self.feed(record, line, None)
            }
            Incoming::Message {
                record: line,
                asks,
                results,
            } => {
                let own = time_of(&line);
                let events = self.feed(record, arrival(line, record), own);
                let questions = events
                    .iter()
                    .filter(|event| event.kind == EventKind::QuestionAsked);
                for (event, call) in questions.zip(&asks) {
                    self.open.bind_event(&event.id.to_string(), call);
                }
                for call in &results {
                    self.open.close_call(call);
                }
                events
            }
            Incoming::Outcome { record: line } => {
                let own = time_of(&line);
                self.feed(record, arrival(line, record), own)
            }
            Incoming::ToolResult { record: line, call } => {
                let own = time_of(&line);
                let events = self.feed(record, arrival(line, record), own);
                self.open.close_call(&call);
                events
            }
            Incoming::Question(question) => {
                self.open.observe_question(question);
                self.count("request/droid.ask_user")
            }
            Incoming::Permission(permission) => {
                self.open.observe_permission(permission);
                self.count("request/droid.request_permission")
            }
            Incoming::PermissionResolved { request_id } => {
                self.open.close_permission(&request_id);
                self.count("permission_resolved")
            }
            Incoming::Protocol(name) => self.count(&name),
            Incoming::Unrecognised(name) => {
                let line = serde_json::json!({ "type": name }).to_string().into_bytes();
                self.feed(record, arrival(line, record), None)
            }
        }
    }

    /// The events of the end of the spool: call it once, when the process has exited.
    pub fn finish(&mut self) -> Vec<Event> {
        let mut events = self.map_session_file(None);
        events.extend(self.mapper.as_mut().map(Mapper::finish).unwrap_or_default());
        events
    }

    fn count(&mut self, name: &str) -> Vec<Event> {
        *self.protocol.entry(name.to_owned()).or_default() += 1;
        Vec::new()
    }

    /// `own` is the time the line itself carries: the session file's records written before it
    /// are mapped first, so the events come in file order. A line with no time of its own (a
    /// turn outcome is timed by its arrival) waits for the next one that has.
    fn feed(&mut self, record: &RawRecord, bytes: Vec<u8>, own: Option<Timestamp>) -> Vec<Event> {
        let (launch, version, template) = (&self.launch, self.version, &self.template);
        self.mapper.get_or_insert_with(|| {
            Mapper::with_template(SessionMeta::root(launch.clone()), version, template.clone())
        });
        let mut events = own.map_or_else(Vec::new, |own| self.map_session_file(Some(own)));
        let mapper = self.mapper.as_mut().expect("the mapper was just created");
        let line = RawRecord {
            ptr: record.ptr.clone(),
            session_id: mapper.session().id.clone(),
            bytes,
            arrived_at: record.arrived_at,
        };
        events.extend(mapper.map(&line));
        events
    }

    /// The events of the session file's records written up to `until`, or all that are left.
    fn map_session_file(&mut self, until: Option<Timestamp>) -> Vec<Event> {
        let (Some(files), Some(mapper)) = (self.files.as_mut(), self.mapper.as_mut()) else {
            return Vec::new();
        };
        let session = mapper.session().id.clone();
        let records = match until {
            Some(until) => files.due(&session, until),
            None => files.rest(&session),
        };
        records
            .into_iter()
            .flat_map(|record| {
                mapper.map(&RawRecord {
                    session_id: session.clone(),
                    ..record
                })
            })
            .collect()
    }
}

fn time_of(line: &[u8]) -> Option<Timestamp> {
    serde_json::from_slice::<Value>(line)
        .ok()?
        .get("timestamp")?
        .as_str()
        .and_then(|text| Timestamp::parse(text).ok())
}

/// A record rebuilt from a notification has the time its message carried, if it carried one;
/// otherwise it takes the time its line arrived, not the time of the line before it.
fn arrival(line: Vec<u8>, record: &RawRecord) -> Vec<u8> {
    let Some(at) = record.arrived_at else {
        return line;
    };
    match serde_json::from_slice::<Value>(&line) {
        Ok(Value::Object(mut fields)) if !fields.contains_key("timestamp") => {
            fields.insert("timestamp".into(), at.to_rfc3339_millis().into());
            Value::Object(fields).to_string().into_bytes()
        }
        _ => line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourcePtr;
    use std::path::PathBuf;

    fn record(offset: u64, bytes: &str) -> RawRecord {
        RawRecord {
            ptr: SourcePtr::Rpc {
                path: PathBuf::from("/spool.jsonl"),
                byte_offset: offset,
                byte_len: bytes.len() as u32,
            },
            session_id: SessionId("launch".into()),
            bytes: bytes.as_bytes().to_vec(),
            arrived_at: None,
        }
    }

    #[test]
    fn a_line_before_the_session_response_takes_the_launch_id() {
        let mut mapper = RpcMapper::new("launch", MappingVersion::V1);
        assert_eq!(mapper.session_id(), None);

        let events = mapper.map(&record(0, r#"{"type":"mystery"}"#));

        assert_eq!(mapper.session_id().unwrap().0, "launch");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0].kind,
            EventKind::Unknown { raw_type } if raw_type == "mystery"
        ));
    }

    #[test]
    fn an_unknown_notification_is_an_unknown_event_and_chatter_is_only_counted() {
        let mut mapper = RpcMapper::new("launch", MappingVersion::V1);
        let wrap = |kind: &str| {
            format!(
                r#"{{"jsonrpc":"2.0","type":"notification","method":"droid.session_notification","params":{{"notification":{{"type":"{kind}"}}}}}}"#
            )
        };

        assert!(
            mapper
                .map(&record(0, &wrap("assistant_text_delta")))
                .is_empty()
        );
        assert!(
            mapper
                .map(&record(1, &wrap("assistant_text_delta")))
                .is_empty()
        );
        let events = mapper.map(&record(2, &wrap("brand_new")));

        assert_eq!(mapper.protocol_counts()["assistant_text_delta"], 2);
        assert!(matches!(
            &events[0].kind,
            EventKind::Unknown { raw_type } if raw_type == "rpc/brand_new"
        ));
        assert!(mapper.open_requests().questions().is_empty());
        assert!(mapper.open_requests_mut().permissions().is_empty());
    }

    #[test]
    fn a_session_response_names_the_session_once() {
        let mut mapper = RpcMapper::new("launch", MappingVersion::V1);
        let start = |id: &str| {
            format!(
                r#"{{"jsonrpc":"2.0","type":"response","id":"1","result":{{"sessionId":"{id}","settings":{{}}}}}}"#
            )
        };

        mapper.map(&record(0, &start("s-1")));
        mapper.map(&record(1, &start("s-2")));

        assert_eq!(mapper.session_id().unwrap().0, "s-1");
    }
}
