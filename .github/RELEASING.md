# Binary releases

Pushes to `main` run `.github/workflows/release.yml`. The workflow tests the
version script, stamps the release version, runs the Rust checks and Matis,
then publishes both `matis` and `matis-migrate` in a GitHub Release.
No crates.io publication or additional Rust dependency is needed.

The initial platform is Linux x86-64 with glibc 2.35 or newer (Ubuntu 22.04 or
newer). Alpine/musl, ARM, macOS, and Windows binaries are not included.

## Versions

The first release uses the package version in `matis-rust/Cargo.toml`, currently
`0.1.0`. Each subsequent release increments the highest stable `vMAJOR.MINOR.PATCH`
tag's patch number. Unrelated tags and prerelease tags do not affect this count.
Rerunning a workflow for an already tagged commit reuses its version.

For a feature or breaking change, update the package version in
`matis-rust/Cargo.toml` and its package entry in `matis-rust/Cargo.lock` to a
version greater than the latest release. The next release uses that version.
Version increments do not infer compatibility from code or commit messages.

Release tags point to the pushed source commit. The workflow stamps Cargo.toml
and Cargo.lock in the build checkout only; it does not commit version changes
to `main`. Thus `matis --version` in a release binary matches the tag, while a
local source build reports the manifest baseline. To reproduce a release build,
check out its tag, run `python3 .github/scripts/release_version.py` with Python
3.11 or newer, then build with `cargo build --manifest-path matis-rust/Cargo.toml
--release --locked --bins`.

The workflow needs GitHub Actions enabled and permission to create tags and
releases with `GITHUB_TOKEN`. Tag rules must permit the workflow to create `v*`
tags. It never pushes commits to `main` and does not require a personal token.
A release stays in draft until both the archive and checksum file are uploaded.
Rerun a failed workflow to finish a partial release.

Releases are serialized and do not cancel a running build. GitHub concurrency
keeps only the newest pending run if several pushes arrive during a build, so
rapid pushes can be combined into one release. Failed validation publishes no
binary. No release is created merely because a tag was pushed.

## Install in another project's CI

Pin a release rather than downloading `latest`. This example requires GitHub CLI
and a Linux x86-64 runner compatible with Ubuntu 22.04:

```yaml
- name: Install Matis
  env:
    GH_TOKEN: ${{ github.token }}
    MATIS_VERSION: v0.1.0
  shell: bash
  run: |
    mkdir -p "$RUNNER_TEMP/matis-download" "$RUNNER_TEMP/matis-bin"
    cd "$RUNNER_TEMP/matis-download"
    gh release download "$MATIS_VERSION" \
      --repo LeonFelipeCordero/matis \
      --pattern 'matis-*-x86_64-unknown-linux-gnu.tar.gz' \
      --pattern SHA256SUMS
    sha256sum --check SHA256SUMS
    tar -xzf matis-*-x86_64-unknown-linux-gnu.tar.gz -C "$RUNNER_TEMP/matis-bin"
    echo "$RUNNER_TEMP/matis-bin" >> "$GITHUB_PATH"
- run: matis --version
- run: matis check --f .
```

No Rust toolchain is needed in the consuming project. Checks without `--save`
do not need a database. Run `matis-migrate` before commands that persist data;
back up an existing database before upgrading.

## Local workflow checks

```sh
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
actionlint .github/workflows/release.yml
```
