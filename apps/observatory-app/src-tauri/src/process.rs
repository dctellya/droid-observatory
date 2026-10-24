#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use tokio::process::Command;

    use super::{group_members, spawn_group};

    const GRACE: Duration = Duration::from_millis(300);

    /// A shell that ignores SIGTERM and keeps two children that inherit the ignore: only a
    /// SIGKILL to the process group ends all three.
    fn stubborn() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "trap '' TERM; sleep 300 & sleep 300 & wait"]);
        command
    }

    fn tame() -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 300 & wait"]);
        command
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn child_process_is_reaped_on_window_close() {
        let hosted = spawn_group(stubborn()).expect("the fake child starts");
        let pgid = hosted.group.pgid;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(pgid, hosted.group.pid, "the child leads its own group");
        assert!(
            group_members(pgid) >= 3,
            "the shell and its two sleeps run in the group"
        );

        let started = Instant::now();
        let reaped = hosted.group.reap(GRACE).await;

        assert!(reaped.killed, "SIGTERM was ignored, so SIGKILL was needed");
        assert_eq!(group_members(pgid), 0, "no member of the group is left");
        assert!(started.elapsed() < GRACE * 4, "{:?}", started.elapsed());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_child_that_exits_on_sigterm_is_not_killed() {
        let hosted = spawn_group(tame()).expect("the fake child starts");
        tokio::time::sleep(Duration::from_millis(200)).await;

        let reaped = hosted.group.reap(Duration::from_secs(5)).await;

        assert!(!reaped.killed);
        assert_eq!(group_members(hosted.group.pgid), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn reaping_a_group_that_is_gone_is_harmless() {
        let hosted = spawn_group(tame()).expect("the fake child starts");
        hosted.group.reap(GRACE).await;

        let again = hosted.group.reap(GRACE).await;

        assert!(!again.killed);
    }
}
