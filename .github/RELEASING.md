# Local releases

Release builds and uploads run locally, never in GitHub Actions. Check workflows
remain enabled, including the release script tests. No release workflow or Actions
artifact upload remains. README is not part of this change.

```sh
bash scripts/release.sh --help
bash scripts/release.sh version
bash scripts/release.sh build ubuntu 0.1.0
bash scripts/release.sh build arch 0.1.0
bash scripts/release.sh build macos 0.1.0
bash scripts/release.sh publish 0.1.0
```

Run each build on its matching OS, using its native architecture. Ubuntu and Arch
must use the same architecture. Build Ubuntu on the oldest supported Ubuntu
release; newer host libraries can prevent installation on older systems.
Cross-compilation and multiple architectures per release are not supported.
Builds require Rust, Bun, Git, Python 3.11+, and the native packaging tool:
`dpkg-deb`, `makepkg`, or `zip`. Arch builds must run as a normal user, not root.
The script installs dashboard dependencies with `bun install --no-save`.

The version command preserves the existing version determination: use the Cargo
baseline for the first release, increment the highest stable tag's patch, or reuse
a stable version tagged at HEAD. A higher Cargo baseline overrides older tags.
Fetch tags before selecting the version. Version selection does not modify source.
Builds stamp Cargo.toml and Cargo.lock in a temporary source copy and verify that
`matis --version` matches. Build committed source with a clean checkout. Build and
publish the same pushed commit on all machines; do not mix package versions or
architectures. The script does not fetch tags, push commits, or sign packages.

Collect exactly these three nonempty files in `artifacts/` on the publishing host:

- `matis-<version>-ubuntu.deb`
- `matis-<version>-arch.pkg.tar.zst`
- `matis-<version>-macos.zip`

Linux packages install `matis`, `matis-migrate`, and the native `matis-ui` executable
under `/usr/bin`, plus the dashboard, icon, license, and `matis.desktop`. Install
with `sudo apt install ./artifacts/matis-<version>-ubuntu.deb` or
`sudo pacman -U artifacts/matis-<version>-arch.pkg.tar.zst`. Open **Matis** from the
application launcher. The executable includes the Bun runtime; users do not need
Bun or Rust. It starts the dashboard on `127.0.0.1:43127` and opens the default
browser. Keep the process running while using the dashboard; the port must be free.
The launcher runs the existing `matis-migrate` tool before starting the dashboard.
The existing user database location remains unchanged. Back up databases before
upgrading.

The macOS zip contains `Matis.app`. Copy it into Applications and open it. CLI
executables are in `Matis.app/Contents/MacOS`. The app is unsigned and not notarized;
macOS may require explicit user approval. A native macOS build is required.

Authenticate `gh` with repository release write access. Publish first creates a
draft `v<version>` at HEAD, uploads the three packages, verifies the complete remote
asset name set, and only then publishes. Failed uploads or asset checks leave a
draft. Rerun publish to resume that draft. Extra local or remote assets for the
version cause failure. Published releases are immutable to this script. Serialize
publishing manually; do not run concurrent publishers for the same version.
No live release is created by tests.

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
bash -n scripts/release.sh
```
