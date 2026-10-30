#!/usr/bin/env python3
"""Tests for the idle time, host health and owner intervention tables of scripts/usage-report.py (owner items 6, 7, 8 of the
M3 boundary). Each test builds a scratch copy of the usage fixture (usage_scratch.py) and appends its own events on 2026-10-26.

The scenario of IdleTime: lead turns, then gaps of 25 min (a 402 at its start), 20 min (connection faults inside), 18 min
(an owner pause and resume), 30 min (nothing), a gap of exactly 15 min and one of 10 min (neither is idle).
"""

import datetime
import unittest

from usage_scratch import FABLE, MISSION, Scratch, at, table

WORKER = "33333333-3333-4333-8333-333333333333"
RESUME_ONLY = "2026-10-26T01:10:00.000Z"


def boottime(booted_at):
    """The text `sysctl kern.boottime` prints for a boot at the given UTC instant."""
    epoch = int(datetime.datetime.fromisoformat(booted_at).replace(tzinfo=datetime.timezone.utc).timestamp())
    return "kern.boottime: { sec = %d, usec = 796010 } test" % epoch


def idle_scenario(s):
    for hh in ("00:00:10", "00:05:00", "00:30:00", "00:50:00", "01:20:00", "01:50:00", "02:00:00", "02:15:00", "02:25:00"):
        s.turn(at(hh), MISSION, FABLE, i=1000)
    s.raw_log(at("00:05:03"), "WARN", "[LLM] Chat route failure", {
        "sessionId": MISSION, "assistantMessageId": "amid-402", "retried": False, "providerRotated": False, "reason": "paymentRequired",
        "error": {"name": "Error", "message": "402 You've reached your 5-hour standard usage limit (resets in 0h 20min)"}})
    s.raw_log(at("00:40:00"), "WARN", "Failed to fetch routing, keeping in-memory values", {"errorName": "ECONNREFUSED"})
    s.raw_log(at("00:41:00"), "ERROR", "[ShutdownCoordinator] Core shutdown step failed",
              {"error": [{"errno": -49, "syscall": "connect", "code": "EADDRNOTAVAIL"}]})
    s.progress("2026-10-26T00:52:00.000Z", "mission_paused")
    s.progress(RESUME_ONLY, "mission_resumed")


class IdleTime(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        idle_scenario(cls.s)
        cls.text = cls.s.run().text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def gaps(self):
        header, rows = table(self.text, "Idle gaps")
        self.assertEqual(header, ["Start", "End", "Minutes", "Cause", "Evidence"])
        return rows

    def test_a_gap_over_fifteen_minutes_is_idle_and_one_of_exactly_fifteen_is_not(self):
        rows = self.gaps()
        self.assertEqual([r["Minutes"] for r in rows], ["25.00", "20.00", "18.00", "30.00"])
        self.assertEqual([r["Start"] for r in rows],
                         [at("00:05:00"), at("00:30:00"), "2026-10-26T00:52:00.000Z", at("01:20:00")])

    def test_each_gap_takes_the_cause_its_evidence_names(self):
        causes = [r["Cause"] for r in self.gaps()]
        self.assertEqual(causes, ["usage limit", "host", "owner", "unexplained"])

    def test_the_evidence_column_names_the_lines_that_explain_a_gap(self):
        rows = self.gaps()
        self.assertIn("paymentRequired", rows[0]["Evidence"])
        self.assertIn("resets in 0h 20min", rows[0]["Evidence"])
        self.assertIn("ECONNREFUSED 1", rows[1]["Evidence"])
        self.assertIn("EADDRNOTAVAIL 1", rows[1]["Evidence"])
        self.assertIn("mission_resumed", rows[2]["Evidence"])
        self.assertEqual(rows[3]["Evidence"], "none")

    def test_the_summary_has_a_row_per_cause_with_gaps_and_minutes(self):
        header, rows = table(self.text, "Idle time by cause")
        self.assertEqual(header, ["Cause", "Gaps", "Minutes"])
        got = {r["Cause"]: (r["Gaps"], r["Minutes"]) for r in rows}
        self.assertEqual(got, {"usage limit": ("1", "25.00"), "host": ("1", "20.00"), "owner": ("1", "18.00"), "runner idle": ("0", "0.00"),
                               "unexplained": ("1", "30.00"), "total": ("4", "93.00")})

    def test_an_unexplained_gap_is_listed_with_its_bounds(self):
        self.assertIn("unexplained gap: %s to %s (30.00 min)" % (at("01:20:00"), at("01:50:00")), self.text)

    def test_the_gap_rule_and_the_events_it_reads_are_stated(self):
        self.assertIn("A gap is the time between two consecutive events", self.text)
        self.assertIn("more than 15 minutes", self.text)

    def test_a_window_without_a_gap_says_so_and_lists_no_unexplained_gap(self):
        s = Scratch()
        try:
            s.turn(at("00:00:10"), MISSION, FABLE, i=1)
            s.turn(at("00:10:00"), MISSION, FABLE, i=1)
            text = s.run().text
        finally:
            s.close()
        self.assertIn("unexplained: none", text)
        self.assertNotIn("unexplained gap:", text)
        _, rows = table(text, "Idle time by cause")
        self.assertEqual(dict((r["Cause"], r["Minutes"]) for r in rows)["total"], "0.00")


class HostHealth(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = Scratch()
        idle_scenario(cls.s)
        s = cls.s
        s.raw_log(at("00:20:00"), "WARN", "[LLM] Chat route failure", {
            "sessionId": MISSION, "assistantMessageId": "amid-net", "retried": True, "providerRotated": False, "reason": "networkError",
            "error": {"name": "LLMHostDurationLimitError", "message": "The response exceeded the 600s host limit"}})
        s.raw_log(at("00:21:00"), "INFO", "[metrics_log_droid_chat_client_success_count]", {
            "sessionId": MISSION, "assistantMessageId": "amid-retry", "retried": True, "providerRotated": False})
        s.raw_log(at("00:22:00"), "INFO", "[metrics_log_droid_chat_client_success_count]", {
            "sessionId": MISSION, "assistantMessageId": "amid-rot", "retried": False, "providerRotated": True})
        s.raw_log(at("00:23:00"), "INFO", "[metrics_log_droid_chat_client_success_count]", {
            "sessionId": MISSION, "assistantMessageId": "amid-ok", "retried": False, "providerRotated": False})
        cls.text = s.run(["--boottime", boottime("2026-10-23T00:00:00")]).text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def rows(self, heading):
        return {r["Kind"]: r for r in table(self.text, heading)[1]}

    def test_the_sysctl_line_is_pasted_and_the_uptime_is_read_against_the_49_7_day_limit(self):
        self.assertIn("sysctl kern.boottime: kern.boottime: { sec = ", self.text)
        header, rows = table(self.text, "Host health")
        self.assertEqual(header, ["Check", "Value"])
        got = {r["Check"]: r["Value"] for r in rows}
        self.assertEqual(got["Uptime at generation"], "6.44 days")
        self.assertEqual(got["Uptime at the window end"], "3.25 days")
        self.assertEqual(got["TCP clock limit"], "49.7 days")
        self.assertEqual(got["Days to the limit at generation"], "43.26")

    def test_stalled_and_retried_calls_are_counted_by_kind_and_the_usage_limit_is_apart(self):
        header, _ = table(self.text, "Model call faults")
        self.assertEqual(header, ["Kind", "Lines", "Calls", "First", "Last"])
        rows = self.rows("Model call faults")
        self.assertEqual((rows["Chat route failure: paymentRequired"]["Lines"], rows["Chat route failure: paymentRequired"]["Calls"]), ("1", "1"))
        self.assertEqual(rows["Chat route failure: networkError"]["Lines"], "1")
        self.assertEqual(rows['retried: true']["Lines"], "2")
        self.assertEqual(rows['providerRotated: true']["Lines"], "1")
        self.assertEqual(rows["ECONNREFUSED"]["Lines"], "1")
        self.assertEqual(rows["EADDRNOTAVAIL"]["Lines"], "1")

    def test_a_call_is_counted_once_across_the_lines_that_name_it(self):
        self.assertIn("stalled or retried model calls, usage limit excluded: 3", self.text)

    def test_the_verdict_is_one_line_that_says_the_host_inflated_the_numbers_and_why(self):
        line = [x for x in self.text.splitlines() if x.startswith("host verdict:")]
        self.assertEqual(len(line), 1)
        self.assertTrue(line[0].startswith("host verdict: inflated"), line[0])
        self.assertIn("20.00 min idle by host faults", line[0])
        self.assertIn("3 stalled or retried calls", line[0])

    def test_an_uptime_past_the_limit_inflates_the_numbers_on_its_own(self):
        s = Scratch()
        try:
            s.turn(at("00:00:10"), MISSION, FABLE, i=1)
            text = s.run(["--boottime", boottime("2026-08-27T00:00:00")]).text
        finally:
            s.close()
        line = [x for x in text.splitlines() if x.startswith("host verdict:")][0]
        self.assertTrue(line.startswith("host verdict: inflated"), line)
        self.assertIn("uptime 60.25 days is past 49.7", line)

    def test_a_clean_window_on_a_young_host_did_not_inflate_the_numbers(self):
        s = Scratch()
        try:
            s.turn(at("00:00:10"), MISSION, FABLE, i=1)
            text = s.run(["--boottime", boottime("2026-10-23T00:00:00")]).text
        finally:
            s.close()
        line = [x for x in text.splitlines() if x.startswith("host verdict:")][0]
        self.assertTrue(line.startswith("host verdict: did not inflate"), line)

    def test_a_host_booted_after_the_window_end_has_no_uptime_for_the_window(self):
        s = Scratch()
        try:
            s.turn(at("00:00:10"), MISSION, FABLE, i=1)
            text = s.run(["--boottime", boottime("2026-10-27T00:00:00")]).text
        finally:
            s.close()
        got = {r["Check"]: r["Value"] for r in table(text, "Host health")[1]}
        self.assertTrue(got["Uptime at the window end"].startswith("booted after the window end"), got)

    def test_a_missing_sysctl_is_reported_and_leaves_the_verdict_to_the_other_evidence(self):
        s = Scratch()
        try:
            s.turn(at("00:00:10"), MISSION, FABLE, i=1)
            text = s.run(["--boottime", "unavailable"]).text
        finally:
            s.close()
        self.assertIn("sysctl kern.boottime: unavailable", text)
        self.assertIn("host verdict: did not inflate", text)


class OwnerInterventions(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.s = s = Scratch()
        s.progress("2026-10-26T01:00:00.000Z", "worker_started", workerSessionId=WORKER, featureId="m9-alpha", modelId="claude-sonnet-5-5")
        s.progress("2026-10-26T01:10:00.000Z", "mission_resumed", resumeWorkerSessionId=WORKER)
        s.progress("2026-10-26T03:00:00.000Z", "mission_resumed")
        s.progress("2026-10-26T03:00:01.000Z", "worker_started", workerSessionId="aaaaaaaa-0000-4000-8000-0000000000b1",
                   featureId="m9-beta", modelId="claude-sonnet-5-5")
        s.progress("2026-10-26T03:41:00.000Z", "worker_started", workerSessionId="aaaaaaaa-0000-4000-8000-0000000000b2",
                   featureId="m9-gamma", modelId="claude-sonnet-5-5")
        s.transcript(MISSION, [
            {"id": "ask1", "timestamp": "2026-10-26T03:30:00.000Z", "message": {"role": "assistant", "content": [
                {"type": "tool_use", "id": "toolu_ask1", "name": "AskUser", "input": {"questionnaire": "Proceed to M4?"}}]}},
            {"id": "ans1", "timestamp": "2026-10-26T03:40:00.000Z", "message": {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_ask1", "content": "Answer: go, with the cheaper plan"}]}},
            {"id": "repl-tool-call:s1-1", "timestamp": "2026-10-26T04:00:00.000Z", "message": {"role": "assistant", "content": [
                {"type": "tool_use", "id": "s1-1", "name": "StartMissionRun", "input": {
                    "message": "Resume the validator", "resumeWorkerSessionId": WORKER,
                    "messageToWorker": "The owner restored the models; go on"}}]}},
            {"id": "plain", "timestamp": "2026-10-26T04:10:00.000Z", "message": {"role": "assistant", "content": [
                {"type": "tool_use", "id": "s2-1", "name": "StartMissionRun", "input": {"message": "Next feature"}}]}},
        ])
        cls.text = s.run().text

    @classmethod
    def tearDownClass(cls):
        cls.s.close()

    def rows(self):
        header, rows = table(self.text, "Owner interventions")
        self.assertEqual(header, ["Time", "Kind", "Feature unblocked", "Detail", "Source"])
        return rows

    def test_the_table_lists_resumes_answers_and_messages_in_time_order_with_the_feature_each_unblocked(self):
        got = [(r["Time"], r["Kind"], r["Feature unblocked"]) for r in self.rows()]
        self.assertEqual(got, [
            ("2026-10-26T01:10:00.000Z", "mission_resumed", "m9-alpha"),
            ("2026-10-26T03:00:00.000Z", "mission_resumed", "m9-beta"),
            ("2026-10-26T03:40:00.000Z", "AskUser answer", "m9-gamma"),
            ("2026-10-26T04:00:00.000Z", "messageToWorker", "m9-alpha"),
        ])

    def test_a_start_mission_run_without_a_message_to_the_worker_is_not_an_intervention(self):
        self.assertNotIn("2026-10-26T04:10:00", "".join(r["Time"] for r in self.rows()))

    def test_the_detail_carries_the_answer_or_message_text_and_the_source_names_the_file(self):
        rows = {r["Kind"]: r for r in self.rows()}
        self.assertIn("go, with the cheaper plan", rows["AskUser answer"]["Detail"])
        self.assertIn("The owner restored the models", rows["messageToWorker"]["Detail"])
        self.assertIn("progress_log.jsonl:", rows["mission_resumed"]["Source"])
        self.assertIn("%s.jsonl" % MISSION, rows["AskUser answer"]["Source"])

    def test_the_count_line_states_the_interventions_by_kind(self):
        self.assertIn("owner interventions: 4 (mission_resumed 2, AskUser answer 1, messageToWorker 1)", self.text)

    def test_an_intervention_ending_a_gap_makes_the_gap_owner_idle(self):
        s = Scratch()
        try:
            s.turn(at("00:00:10"), MISSION, FABLE, i=1)
            s.turn(at("00:40:00"), MISSION, FABLE, i=1)
            s.transcript(MISSION, [
                {"id": "a", "timestamp": at("00:00:20"), "message": {"role": "assistant", "content": [
                    {"type": "tool_use", "id": "t1", "name": "AskUser", "input": {"questionnaire": "q"}}]}},
                {"id": "b", "timestamp": at("00:39:59"), "message": {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "t1", "content": "yes"}]}}])
            text = s.run().text
        finally:
            s.close()
        _, rows = table(text, "Idle gaps")
        self.assertEqual([(r["Cause"], r["Minutes"]) for r in rows], [("owner", "39.83")])


if __name__ == "__main__":
    unittest.main()
