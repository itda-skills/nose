use super::*;

/// A git command rooted at `root`, with inherited git env vars cleared so it always
/// operates on `root`'s repo — not on a `GIT_DIR`/`GIT_WORK_TREE` set by an outer hook.
fn git(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    git_cmd()
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("failed to run git (is it installed and on PATH?)")
}

fn git_cmd() -> Command {
    let mut cmd = Command::new("git");
    cmd.env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_COMMON_DIR");
    cmd
}

pub(super) fn git_repo_root() -> Result<PathBuf> {
    let out = git_cmd()
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git (is it installed and on PATH?)")?;
    if !out.status.success() {
        anyhow::bail!("not inside a git repository");
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

/// A throwaway worktree checked out at `base`, removed on drop.
pub(super) struct BaseWorktree {
    root: PathBuf,
    pub(super) path: PathBuf,
}

impl BaseWorktree {
    pub(super) fn create(root: &Path, base: &str) -> Result<Self> {
        // Unique per invocation (pid alone can be reused, racing parallel runs on the path).
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("nose-review-{}-{nonce}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        // Clear any stale registration from a previously-killed run at this path.
        let _ = git(root, &["worktree", "prune"]);
        let out = git(
            root,
            &[
                "worktree",
                "add",
                "--detach",
                "--quiet",
                &path.to_string_lossy(),
                base,
            ],
        )?;
        if !out.status.success() {
            anyhow::bail!(
                "could not check out base `{base}`: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Ok(Self {
            root: root.to_path_buf(),
            path,
        })
    }
}

impl Drop for BaseWorktree {
    fn drop(&mut self) {
        let _ = git(
            &self.root,
            &[
                "worktree",
                "remove",
                "--force",
                &self.path.to_string_lossy(),
            ],
        );
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Resolve user paths once, relative to the caller's cwd, into repo-relative
/// pathspecs. Git commands run with `-C <repo-root>`, so passing the raw user
/// path would reinterpret `src` from a nested cwd as `<repo>/src`.
pub(super) fn repo_relative_paths(paths: &[PathBuf], root: &Path) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|p| match canonical(p).strip_prefix(root) {
            Ok(rel) if rel.as_os_str().is_empty() => PathBuf::from("."),
            Ok(rel) => rel.to_path_buf(),
            Err(_) => p.clone(),
        })
        .collect()
}

/// Re-root each repo-relative path under the base worktree so detection scans
/// the base copy of the same files the diff pathspec selected.
pub(super) fn reroot_paths(paths: &[PathBuf], base: &Path) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|p| {
            if p.is_absolute() {
                p.clone()
            } else {
                base.join(p)
            }
        })
        .collect()
}

/// Changed line ranges on the **base (old) side**, keyed by repo-relative path.
pub(super) fn git_changed_ranges(
    root: &Path,
    base: &str,
    paths: &[PathBuf],
) -> Result<HashMap<String, Vec<(u32, u32)>>> {
    let mut argv: Vec<String> = ["diff", "--unified=0", "--no-color", base]
        .iter()
        .map(|s| s.to_string())
        .collect();
    if !paths.is_empty() {
        argv.push("--".into());
        for p in paths {
            argv.push(p.to_string_lossy().into_owned());
        }
    }
    let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
    let out = git(root, &refs)?;
    if !out.status.success() {
        anyhow::bail!(
            "`git diff {base}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(parse_old_side_ranges(&String::from_utf8_lossy(&out.stdout)))
}

/// Parse `git diff --unified=0` text into base-side changed line ranges per repo-relative
/// path. Pure (no git) so it can be unit-tested against crafted diff output.
pub(super) fn parse_old_side_ranges(diff: &str) -> HashMap<String, Vec<(u32, u32)>> {
    let mut map: HashMap<String, Vec<(u32, u32)>> = HashMap::new();
    let mut current: Option<String> = None;
    // `--- a/path` is the base-file header, but a *deleted* line whose content starts with
    // "-- " also renders as "--- …" in the hunk body. They're disambiguated structurally:
    // the file header sits in the per-file block before the first `@@`; once hunks begin a
    // "--- " line is body content. `diff --git` resets the block for the next file.
    let mut in_hunks = false;
    for line in diff.lines() {
        if line.starts_with("diff --git") {
            in_hunks = false;
            current = None;
        } else if !in_hunks && line.starts_with("--- ") {
            // "--- a/path" → base-side path; "--- /dev/null" (added file) → no base member
            current = line
                .strip_prefix("--- ")
                .and_then(|r| r.strip_prefix("a/"))
                .map(|p| p.to_string());
        } else if line.starts_with("@@") {
            in_hunks = true;
            if let (Some(file), Some((start, count))) = (&current, parse_hunk_old(line)) {
                // count == 0 is a pure insertion *after* base line `start` (no base line
                // changed): encode the gap as `(start+1, start)` so it touches only members
                // that straddle it, not one that merely ends at `start`.
                let range = if count == 0 {
                    (start + 1, start)
                } else {
                    (start, start + count - 1)
                };
                map.entry(file.clone()).or_default().push(range);
            }
        }
    }
    map
}

/// Parse the old-side range from a hunk header `@@ -a,b +c,d @@ ...` → `(a, b)`, where a
/// missing `,b` means a count of 1.
fn parse_hunk_old(line: &str) -> Option<(u32, u32)> {
    let after_minus = line.split('-').nth(1)?;
    let spec = after_minus.split([' ', '+']).next()?.trim();
    let mut parts = spec.split(',');
    let start: u32 = parts.next()?.parse().ok()?;
    let count: u32 = match parts.next() {
        Some(c) => c.parse().ok()?,
        None => 1,
    };
    Some((start, count))
}

/// Best-effort absolute, symlink-resolved path.
pub(super) fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}
