# Local releases

Run this single command on native Ubuntu or Arch Linux:

```sh
bash scripts/release.sh
```

The script detects the distro from `ID` or `ID_LIKE`, checks `gh auth status`,
fetches `origin --tags`, selects and validates the version, builds one native
package, then publishes it. Other hosts fail before authentication, fetching,
building, or publishing. No containers, SSH, macOS builds, cross-compilation, or
extra host configuration are used. Release builds and uploads never run in
GitHub Actions; check workflows remain enabled.

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
Published releases cannot be changed by this script. Serialize releases manually;
do not run concurrent publishers for the same version.

Optional commands support debugging and draft recovery. They are not required
for the normal release:

```sh
bash scripts/release.sh --help
bash scripts/release.sh version
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
