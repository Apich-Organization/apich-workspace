# apich-vcs

A version control tool: continuous FastCDC-chunked snapshots, weave-free branch merging, and a
bidirectional Git bridge. It works standalone on any local directory — no server, no database, no
containers. It is also used as a library by the APICH web application, but that is optional: the
`apich` CLI below has no dependency on it.

Only two command groups need a network peer at all: `apich git ...` (talking to a real Git
remote) and `apich remote ...` (talking to another apich-vcs installation, e.g. an APICH web
server's `/vcs-remote/...` endpoint). Every other command works fully offline.

## Install

From this repository:

```bash
cargo install --path crates/apich-vcs
```

This installs a single binary, `apich`, onto your `PATH`. Requires Rust and, for `apich git`
commands, a real `git` installation. GPG-signing/verifying snapshots (`snapshot --sign`,
`verify`) requires `gpg` on `PATH`; it is optional otherwise.

## Quick start

```bash
mkdir my-project && cd my-project
apich init                          # or just skip this -- any command auto-initializes
echo "hello" > notes.txt
apich status                        # see what changed
apich snapshot -m "Initial notes"   # commit an immutable, deduplicated snapshot
apich log                           # see snapshot history
apich branch create experiment
apich checkout experiment
apich branch                        # list branches, current one marked with *
apich merge main                    # weave-free 3-way merge back into the current branch
```

`-p`/`--path <dir>` (before the subcommand) targets a different directory than the current one;
every example above assumes you're already inside the repository.

## Command reference

Run `apich --help` or `apich <command> --help` for the full, current list of flags — this is a
summary, not a substitute.

**Local, no network needed:**
- `init`, `status`, `snapshot [--sign [KEY_ID]]`, `verify <id> --pubkey <file>`, `log`, `show
  <id>`, `cat <file> [--snapshot <id>]`, `diff`
- `branch [list|create|switch]`, `checkout <branch>`, `merge <branch>`, `revert <id>`
- `undo`, `redo`, `oplog` — every mutating operation is reversible via a local operation log
- `milestone <name>`, `milestones` — tag a snapshot for easy reference (e.g. `v1.0`)
- `gc` — prune old snapshots per retention policy and sweep now-unreferenced chunks
- `bundle export/import` — package the whole repository (all snapshots, branches, history) into
  a single `.apich-bundle` file for backup or offline transfer; `bundle export-archive` exports
  one snapshot's files as a plain `.tar.gz` with no apich-vcs metadata (for submission/sharing)
- `config show/ignore-add/ignore-remove/lfs-add/lfs-threshold` — inspect and edit repository
  configuration
- `material clone/list` — vendor an external Git repository into a subfolder as tracked material

**Talk to a real Git remote (needs `git` + network):**
- `git init`, `git export`, `git remote-add`, `git remotes`, `git push/pull/fetch/rebase`,
  `git clone <url> [dest]` (top-level `apich clone`) — every project is simultaneously a native
  apich-vcs repository and, once `git init`/`git export` has run, a real Git repository; these
  subcommands drive that real Git repository with the real `git` binary. Auth (SSH agent,
  stored HTTPS credentials, credential helpers) is whatever your local `git` is already
  configured with — apich-vcs doesn't reimplement Git auth.

**Talk to another apich-vcs installation (needs network, e.g. an APICH web server):**
- `remote clone <url> [dest] [--token <PAT>]`, `remote push <url> [--token <PAT>]`,
  `remote pull <url> [--token <PAT>]` — apich-vcs's own protocol, distinct from the Git bridge
  above. `--token` can also come from the `APICH_TOKEN` environment variable. `push`/`pull` are
  fast-forward only: a diverged push is rejected and reported, never silently discarded.

## Ignore rules

Yes: a `.apichignore` file in the repository root is loaded automatically, same `.gitignore`
glob syntax, alongside `.gitignore` itself (both are read if present -- you don't need to choose
one). There's a second, separate mechanism too: `apich config ignore-add <pattern>`/`ignore-remove
<pattern>` writes patterns into the repository's own `.apich/config.toml` instead of a plain-text
ignore file -- useful for a rule you want versioned with the rest of the repo's apich-vcs config
(`apich config show` to see the current merged set) rather than living in its own file. Both feed
the same underlying filter, along with built-in profiles (Academic/Python/R/Development, e.g.
`__pycache__/`, `.Rhistory`, LaTeX build artifacts) enabled by default -- see `IgnoreProfile` in
`src/ignore/profile.rs` for the exact built-in list.

## GPG-signed snapshots

```bash
apich snapshot -m "Submitted draft" --sign            # sign with gpg's configured default key
apich snapshot -m "Submitted draft" --sign <KEY_ID>    # sign with a specific key
apich verify <snapshot-id> --pubkey alice-pubkey.asc   # verify against one specific public key
```

Signing always runs against your own local `gpg` keyring — a private key is never read or
transmitted by apich-vcs itself, only the resulting detached signature is stored on the snapshot.
Verification runs in an isolated, throwaway keyring seeded only with the one public key you pass
in, so a result reflects trust in that specific key, not whatever else is in your default keyring.
