//! The hosted child and its process group. Droid starts workers and shell commands of its own,
//! so closing the window has to end the group, not the one pid: the child leads a group of its
//! own and the group is signalled as a whole.

use std::io;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::sync::watch;

/// How often `reap` looks at the group while it waits for it to end.
const REAP_POLL: Duration = Duration::from_millis(25);

/// A started child: its pipes, the group it leads and the receiver of its exit code. The child
/// itself is owned by a task that waits for it, so an exited child never stays a zombie in the
/// group.
pub struct Spawned {
    pub group: ProcessGroup,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    /// `None` while the child runs; `Some(code)` once it has exited (`Some(None)` when a signal
    /// ended it).
    pub exit: watch::Receiver<Option<Option<i32>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessGroup {
    pub pid: u32,
    pub pgid: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reaped {
    /// SIGTERM was not enough: some member needed SIGKILL.
    pub killed: bool,
}

/// Starts `command` as the leader of a new process group with piped stdin and stdout. Stderr
/// is the caller's to set.
pub fn spawn_group(mut command: Command) -> io::Result<Spawned> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .process_group(0);
    let mut child = command.spawn()?;
    let pid = child
        .id()
        .ok_or_else(|| io::Error::other("the child exited before its pid was read"))?;
    let (sender, exit) = watch::channel(None);
    let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
    tokio::spawn(async move {
        let code = child.wait().await.ok().and_then(|status| status.code());
        let _ = sender.send(Some(code));
    });
    Ok(Spawned {
        group: ProcessGroup { pid, pgid: pid },
        stdin,
        stdout,
        exit,
    })
}

impl ProcessGroup {
    /// SIGTERM to the group, then up to `grace` for it to end, then SIGKILL. Safe to repeat and
    /// safe on a group that is gone.
    pub async fn reap(&self, grace: Duration) -> Reaped {
        if group_members(self.pgid) == 0 {
            return Reaped { killed: false };
        }
        signal(self.pgid, "TERM").await;
        if wait_empty(self.pgid, grace).await {
            return Reaped { killed: false };
        }
        signal(self.pgid, "KILL").await;
        wait_empty(self.pgid, grace).await;
        Reaped { killed: true }
    }
}

async fn wait_empty(pgid: u32, within: Duration) -> bool {
    let deadline = Instant::now() + within;
    loop {
        if group_members(pgid) == 0 {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(REAP_POLL).await;
    }
}

/// `kill` rather than a libc call: the shell crate forbids unsafe code and a signal to a group
/// is the one thing that needs it.
async fn signal(pgid: u32, name: &str) {
    let _ = Command::new("/bin/kill")
        .args(["-s", name, "--", &format!("-{pgid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
}

/// The processes whose group is `pgid`, zombies included: a zombie is still in the group until
/// its parent waits for it.
pub fn group_members(pgid: u32) -> usize {
    let Ok(output) = std::process::Command::new("ps")
        .args(["-axo", "pgid="])
        .output()
    else {
        return 0;
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.trim().parse::<u32>().is_ok_and(|id| id == pgid))
        .count()
}

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
