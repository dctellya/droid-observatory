//! Child discovery, the session tree, role resolution and the sidecar, over the M0 capture
//! and over small synthetic session directories (architecture §4.2, §5).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{JsonlSource, SessionTree, Source, read_sidecar};
use observatory_core::model::{Role, SessionId};
use serde_json::{Value, json};

const M0_SLUG: &str = "2026-10-22-observatory-v1-m0";

fn streams_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/streams")
}

fn m0_root() -> PathBuf {
    streams_dir().join(format!("{M0_SLUG}.jsonl"))
}

fn m0_header() -> Value {
    let text = std::fs::read_to_string(streams_dir().join(format!("{M0_SLUG}.capture.json")))
        .expect("the M0 header exists");
    serde_json::from_str(&text).expect("the M0 header is JSON")
}

fn m0_tree() -> SessionTree {
    SessionTree::discover(&m0_root(), &IngestConfig::default()).expect("the M0 capture opens")
}

fn id(text: &str) -> SessionId {
    SessionId(text.to_owned())
}

fn first_line(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(text.lines().next().unwrap()).unwrap()
}

#[test]
fn child_sessions_attach_to_their_parent() {
    let tree = m0_tree();
    let header = m0_header();
    let expected = header["sessions"].as_array().unwrap();

    assert_eq!(tree.sessions().len(), expected.len());
    assert_eq!(tree.root().id.0, expected[0]["id"].as_str().unwrap());
    assert_eq!(tree.root().parent, None);
    assert_eq!(tree.mission_id().0, tree.root().id.0);

    let mut direct_children_of_root = 0;
    for entry in &expected[1..] {
        let session = tree
            .get(&id(entry["id"].as_str().unwrap()))
            .unwrap_or_else(|| panic!("{} is in the tree", entry["id"]));
        let start = first_line(&streams_dir().join(entry["file"].as_str().unwrap()));
        assert_eq!(
            session.parent.as_ref().map(|parent| parent.0.as_str()),
            start["callingSessionId"].as_str(),
            "{}",
            session.id
        );
        assert_eq!(
            session.calling_tool_use_id.as_deref(),
            start["callingToolUseId"].as_str(),
            "{}",
            session.id
        );
        let parent = session.parent.as_ref().expect("a child has a parent");
        assert!(
            tree.get(parent).is_some(),
            "{} has a known parent",
            session.id
        );
        assert!(tree.children_of(parent).any(|child| child.id == session.id));
        direct_children_of_root += usize::from(*parent == tree.root().id);
    }
    assert_eq!(
        tree.children_of(&tree.root().id).count(),
        direct_children_of_root
    );
    assert!(
        direct_children_of_root < expected.len() - 1,
        "validator sessions spawn children of their own, so the tree is two levels deep"
    );
}

#[test]
fn discovery_finds_children_beside_the_root_and_beneath_it_and_skips_strangers() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, lines: &[Value]| {
        let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
        std::fs::write(dir.path().join(name), text).unwrap();
    };
    let start = |id: &str, caller: Option<&str>| json!({"type": "session_start", "id": id, "callingSessionId": caller, "callingToolUseId": null});
    write("root.jsonl", &[start("root", None)]);
    write("beside.jsonl", &[start("beside", Some("root"))]);
    write("grandchild.jsonl", &[start("grandchild", Some("beside"))]);
    write(
        "stranger.jsonl",
        &[start("stranger", Some("somebody-else"))],
    );
    write("other-root.jsonl", &[start("other-root", None)]);
    std::fs::write(dir.path().join("notes.txt"), "not a session").unwrap();
    std::fs::create_dir(dir.path().join("root.children")).unwrap();
    std::fs::write(
        dir.path().join("root.children").join("below.jsonl"),
        format!("{}\n", start("below", Some("root"))),
    )
    .unwrap();

    let tree =
        SessionTree::discover(&dir.path().join("root.jsonl"), &IngestConfig::default()).unwrap();

    let mut ids: Vec<_> = tree.sessions().iter().map(|s| s.id.0.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, ["below", "beside", "grandchild", "root"]);
    assert_eq!(
        tree.get(&id("grandchild")).unwrap().parent,
        Some(id("beside"))
    );
    assert_eq!(tree.root().id, id("root"));
}

#[test]
fn opening_a_tree_reads_the_root_and_every_child_file() {
    let tree = m0_tree();
    let mut source = JsonlSource::open_tree(&tree, &IngestConfig::default()).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..100_000 {
        for record in source.next_batch().unwrap() {
            seen.insert(record.session_id);
        }
        if source.at_eof() {
            break;
        }
    }
    assert!(source.at_eof());
    assert_eq!(seen.len(), tree.sessions().len());
    assert_eq!(source.root_session_id(), &tree.root().id);
}

fn header_roles() -> BTreeMap<String, String> {
    m0_header()["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["id"].as_str().unwrap().to_owned(),
                entry["role"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn role_name(role: Role) -> String {
    serde_json::to_value(role)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn roles_resolve_for_orchestrator_worker_and_both_validators() {
    let tree = m0_tree();
    let expected = header_roles();

    let resolved: BTreeMap<String, String> = tree
        .sessions()
        .iter()
        .map(|session| (session.id.0.clone(), role_name(session.role)))
        .collect();

    assert_eq!(resolved, expected);
    assert!(tree.sessions().iter().all(|s| s.role != Role::Unresolved));
    let count = |role: &str| resolved.values().filter(|r| r.as_str() == role).count();
    assert_eq!(count("orchestrator"), 1);
    assert_eq!(count("worker"), 39);
    assert_eq!(count("scrutiny_validator"), 7);
    assert_eq!(count("user_testing_validator"), 9);
}

#[test]
fn a_child_whose_role_cannot_be_resolved_stays_unresolved() {
    let dir = tempfile::tempdir().unwrap();
    let parent = json!({"type": "session_start", "id": "root"});
    let task_use = json!({"type": "message", "message": {"role": "assistant", "content": [
        {"type": "tool_use", "id": "t-odd", "name": "Task", "input": {"subagent_type": "something-new"}},
        {"type": "tool_use", "id": "t-worker", "name": "Task", "input": {"subagent_type": "worker"}}
    ]}});
    std::fs::write(
        dir.path().join("root.jsonl"),
        format!("{parent}\n{task_use}\n"),
    )
    .unwrap();
    let child = |name: &str, tool_use: Option<&str>| {
        let start = json!({"type": "session_start", "id": name, "callingSessionId": "root",
                           "callingToolUseId": tool_use});
        std::fs::write(
            dir.path().join(format!("{name}.jsonl")),
            format!("{start}\n"),
        )
        .unwrap();
    };
    child("odd-type", Some("t-odd"));
    child("missing-tool-use", Some("t-gone"));
    child("no-tool-use-no-tag", None);
    child("plain-worker", Some("t-worker"));

    let tree =
        SessionTree::discover(&dir.path().join("root.jsonl"), &IngestConfig::default()).unwrap();

    let role = |name: &str| tree.get(&id(name)).unwrap().role;
    assert_eq!(role("root"), Role::Orchestrator);
    assert_eq!(role("plain-worker"), Role::Worker);
    assert_eq!(role("odd-type"), Role::Unresolved);
    assert_eq!(role("missing-tool-use"), Role::Unresolved);
    assert_eq!(role("no-tool-use-no-tag"), Role::Unresolved);
}

#[test]
fn a_runner_started_session_takes_its_role_from_the_skill_it_invokes() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("root.jsonl"),
        format!("{}\n", json!({"type": "session_start", "id": "root"})),
    )
    .unwrap();
    let runner = |name: &str, skills: &[&str]| {
        let start = json!({"type": "session_start", "id": name, "callingSessionId": "root",
                           "callingToolUseId": null});
        let mut text = format!("{start}\n");
        for skill in skills {
            let message = json!({"type": "message", "message": {"role": "assistant", "content": [
                {"type": "tool_use", "id": format!("u-{skill}"), "name": "Skill", "input": {"skill": skill}}
            ]}});
            text.push_str(&format!("{message}\n"));
        }
        std::fs::write(dir.path().join(format!("{name}.jsonl")), text).unwrap();
        let sidecar = json!({"tags": [
            {"name": "exec"},
            {"name": "mission-session", "metadata": {"role": "worker", "missionId": "m"}}
        ]});
        std::fs::write(
            dir.path().join(format!("{name}.settings.json")),
            sidecar.to_string(),
        )
        .unwrap();
    };
    runner("scrutiny", &["mission-worker-base", "scrutiny-validator"]);
    runner("user-testing", &["user-testing-validator"]);
    runner("builder", &["mission-worker-base", "core-worker"]);

    let tree =
        SessionTree::discover(&dir.path().join("root.jsonl"), &IngestConfig::default()).unwrap();

    let role = |name: &str| tree.get(&id(name)).unwrap().role;
    assert_eq!(role("scrutiny"), Role::ScrutinyValidator);
    assert_eq!(role("user-testing"), Role::UserTestingValidator);
    assert_eq!(role("builder"), Role::Worker);
}

fn json_u64(value: &Value) -> u64 {
    value.as_u64().unwrap_or(0)
}

#[test]
fn sidecar_usage_model_and_tags_are_read_from_settings_json() {
    let tree = m0_tree();
    let header = m0_header();

    for entry in header["sessions"].as_array().unwrap() {
        let jsonl = streams_dir().join(entry["file"].as_str().unwrap());
        let raw: Value = serde_json::from_str(
            &std::fs::read_to_string(jsonl.with_extension("settings.json")).unwrap(),
        )
        .unwrap();
        let session = tree.get(&id(entry["id"].as_str().unwrap())).unwrap();
        let sidecar = session
            .sidecar
            .as_ref()
            .expect("every M0 session has a sidecar");

        let usage = &raw["tokenUsage"];
        let reported = session.usage().expect("every M0 session reports usage");
        assert_eq!(reported.input, json_u64(&usage["inputTokens"]));
        assert_eq!(reported.output, json_u64(&usage["outputTokens"]));
        assert_eq!(reported.cache_read, json_u64(&usage["cacheReadTokens"]));
        assert_eq!(
            reported.cache_write,
            json_u64(&usage["cacheCreationTokens"])
        );
        assert_eq!(reported.credits, usage["factoryCredits"].as_f64());
        let own = sidecar.token_usage.as_ref().unwrap();
        assert_eq!(own.thinking_tokens, json_u64(&usage["thinkingTokens"]));

        assert_eq!(session.model(), raw["model"].as_str());
        assert_eq!(
            sidecar.reasoning_effort.as_deref(),
            raw["reasoningEffort"].as_str()
        );
        assert_eq!(
            sidecar.interaction_mode.as_deref(),
            raw["interactionMode"].as_str()
        );
        assert_eq!(
            sidecar.autonomy_level.as_deref(),
            raw["autonomyLevel"].as_str()
        );
        assert_eq!(
            sidecar.assistant_active_time_ms,
            raw["assistantActiveTimeMs"].as_u64()
        );
        assert_eq!(
            sidecar
                .inclusive_token_usage
                .as_ref()
                .map(|u| u.input_tokens),
            raw["inclusiveTokenUsage"]["inputTokens"].as_u64()
        );
        assert_eq!(
            sidecar
                .last_call_token_usage
                .as_ref()
                .map(|u| u.output_tokens),
            raw["lastCallTokenUsage"]["outputTokens"].as_u64()
        );
        let children = raw["childInclusiveTokenUsageBySessionId"].as_object();
        assert_eq!(
            sidecar.child_inclusive_token_usage_by_session_id.len(),
            children.map_or(0, serde_json::Map::len),
            "{}",
            session.id
        );
        for (child, usage) in children.into_iter().flatten() {
            assert_eq!(
                sidecar.child_inclusive_token_usage_by_session_id[child].output_tokens,
                json_u64(&usage["outputTokens"])
            );
        }

        let names: Vec<_> = session.tags().iter().map(|tag| tag.name.as_str()).collect();
        let raw_names: Vec<_> = raw["tags"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|tag| tag["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, raw_names);
        assert_eq!(session.first_tag(), raw_names.first().copied());
    }

    let root_tag = &tree.root().tags()[0];
    assert_eq!(root_tag.name, "mission-session");
    assert_eq!(root_tag.metadata.as_ref().unwrap()["role"], "orchestrator");
}

#[test]
fn a_missing_sidecar_is_none_and_unknown_keys_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let jsonl = dir.path().join("s.jsonl");
    std::fs::write(&jsonl, "").unwrap();
    assert!(read_sidecar(&jsonl).unwrap().is_none());

    std::fs::write(
        dir.path().join("s.settings.json"),
        json!({
            "model": "claude-sonnet-5-5",
            "systemPrompt": "never kept",
            "futureKey": {"a": 1},
            "tokenUsage": {"inputTokens": 3, "factoryCredits": 1.5},
            "tags": [{"name": "read-only", "metadata": null}, {"name": "bare"}]
        })
        .to_string(),
    )
    .unwrap();

    let sidecar = read_sidecar(&jsonl).unwrap().expect("the sibling exists");
    assert_eq!(sidecar.model.as_deref(), Some("claude-sonnet-5-5"));
    let usage = sidecar.token_usage.unwrap();
    assert_eq!((usage.input_tokens, usage.output_tokens), (3, 0));
    assert_eq!(usage.factory_credits, Some(1.5));
    let tags = sidecar.tags.unwrap();
    assert_eq!(tags[0].metadata, None);
    assert_eq!(tags[1].name, "bare");

    std::fs::write(dir.path().join("s.settings.json"), "{not json").unwrap();
    assert!(read_sidecar(&jsonl).is_err());
}
