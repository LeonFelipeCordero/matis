# Automatic main releases

`Trusted CI` runs for owner-triggered main pushes and same-repository pull
requests targeting main. It uses read-only contents permission and an ephemeral
runner. Existing hosted workflows still cover other actors and forks.

After every distinct successful main push checked by
`.github/workflows/trustedCi.yml`, `Release` checks out that exact successful
SHA on hosted Ubuntu 24.04 and publishes a native `.deb`. This includes CI-only
commits. PRs, forks, failed runs, and other source workflows cannot publish.
The source actor ID, actor, and triggering actor must also match the owner;
GitHub can report success when an unauthorized source run skips every job.
The release workflow must exist on the default branch before GitHub can trigger
it. Publishing is serialized with cancellation disabled and `queue: max`, so
new runs do not replace pending releases. GitHub allows 100 pending runs; rerun
any release canceled because that platform queue limit was reached.

The workflow fetches full history and tags, then uses the existing version
selection, Ubuntu build, and publish commands. Each untagged HEAD gets the next
patch version, even when only CI changed. An already-published release skips
successfully only when its remote tag resolves to the exact checked SHA. Drafts
resume; mismatched tags, ambiguous draft targets, API failures, upload failures,
and asset mismatches fail without modifying a published release. A draft with
no remote tag must target the full checked SHA. Annotated tags use their peeled
commit. No tag is moved or force-pushed.

Only the hosted release job has `contents: write`. Its token is available to
checkout and the status/publish steps, not the package build or trusted runner
image. Builds use the existing Rust/Bun tools and package layout. Local native
Arch releases remain supported; coordinate manual publishers with automation
because workflow concurrency does not lock local commands.

# Local releases

Run this single command on native Ubuntu or Arch Linux:

```sh
bash scripts/release.sh
```

The script detects the distro from `ID` or `ID_LIKE`, checks `gh auth status`,
fetches `origin --tags`, selects and validates the version, builds one native
package, then publishes it. Other hosts fail before authentication, fetching,
building, or publishing. No containers, SSH, macOS builds, cross-compilation, or
extra host configuration are used by the local command. The automatic workflow
uses the same native Ubuntu build and publish commands on a hosted runner.

Builds require Rust, Bun, Git, Python 3.11+, authenticated GitHub CLI with release
write access, and the native packaging tool: `dpkg-deb` on Ubuntu or `makepkg`
on Arch. Run Arch builds as a normal user, not root. Build Ubuntu on the oldest
supported Ubuntu release; newer host libraries can prevent installation on older
systems. The script installs dashboard dependencies with `bun install --no-save`.

Version selection uses the Cargo baseline for the first release, increments the
highest stable tag's patch, or reuses a stable version tagged at HEAD. A higher
Cargo baseline overrides older tags. Builds stamp Cargo.toml and Cargo.lock in a
temporary source copy and verify that `matis --version` matches. Working-tree
Cargo files remain unchanged. Use a clean checkout at a pushed source commit:
the script includes local edits in builds but does not commit or push them.

The output is one file for the current distro and native architecture:

- Ubuntu: `artifacts/matis-<version>-ubuntu.deb`
- Arch: `artifacts/matis-<version>-arch.pkg.tar.zst`

Packages install `matis`, `matis-migrate`, and the native `matis-ui` executable
under `/usr/bin`, plus the dashboard, icon, license, and `matis.desktop`. Install
with `sudo apt install ./artifacts/matis-<version>-ubuntu.deb` or
`sudo pacman -U artifacts/matis-<version>-arch.pkg.tar.zst`. Open **Matis** from
the application launcher. The executable includes the Bun runtime; users do not
need Bun or Rust. It runs the existing database migration, starts the dashboard
on `127.0.0.1:43127`, and opens the default browser. Keep the process running;
the port must be free. The existing database location remains unchanged. Back up
databases before upgrading.

Publishing creates or resumes a draft `v<version>` at HEAD, uploads only the
current distro's nonempty package, verifies that the remote release contains
exactly that one asset, then publishes. Other local distro packages are not
required or uploaded. Failed uploads or remote asset checks leave a draft.
Published releases cannot be changed by this script. The status command is
read-only and returns `missing`, `draft`, or `published` after checking the exact
HEAD target. API and remote lookup errors are not treated as missing releases.
Serialize manual releases; do not run concurrent publishers for the same version.

Optional commands support debugging and draft recovery. They are not required
for the normal release:

```sh
bash scripts/release.sh --help
bash scripts/release.sh version
bash scripts/release.sh status 0.1.0
bash scripts/release.sh build ubuntu 0.1.0
bash scripts/release.sh build arch 0.1.0
bash scripts/release.sh publish 0.1.0
bash scripts/release.sh publish 0.1.0 arch
```

Build and publish require the matching native distro. Publish detects the distro
when its optional platform argument is absent. The standalone version command
reads local tags without fetching; the no-argument command fetches first.

Tests mock GitHub release calls and build tools; they never publish externally:

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
bash -n scripts/release.sh
```
