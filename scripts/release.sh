#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "$*" >&2; exit 1; }

help() {
  cat <<'HELP'
Usage: bash scripts/release.sh

Detect native Ubuntu/Arch, check gh authentication, fetch origin tags, select
and validate the version, build one native package, then publish it.
Optional debugging commands:
       bash scripts/release.sh version
       bash scripts/release.sh build <ubuntu|arch> <version>
       bash scripts/release.sh publish <version> [ubuntu|arch]

Local builds only, on the matching distro and native architecture.
Build Ubuntu on the oldest supported Ubuntu release; binaries require that
release's system libraries. Requires Rust, Bun, Python 3.11+, Git, authenticated
gh, and dpkg-deb on Ubuntu or makepkg on Arch (run as a normal user).
No new application dependencies are installed by this script.
Dashboard dependencies are installed with bun install --no-save.
version uses existing stable tags and the Cargo baseline, without editing files.
build stamps Cargo files in a temporary source copy, not the working tree.
Output: artifacts/matis-<version>-ubuntu.deb OR -arch.pkg.tar.zst.
Packages install matis, matis-migrate, matis-ui and an application launcher.
The native matis-ui executable serves the dashboard on loopback port 43127
and opens the default browser. It runs until stopped; port must be free.
publish requires a pushed source commit. It creates or resumes a draft
v<version>, uploads one package, checks the exact remote asset set, then
publishes. It refuses changes to published releases. No Actions storage.
HELP
}

validate_version() {
  [[ $1 =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || fail 'Expected stable major.minor.patch version'
}

detect_platform() {
  [[ $(uname -s) == Linux ]] || fail 'Only native Ubuntu/Arch Linux is supported'
  [[ -r /etc/os-release ]] || fail 'Missing /etc/os-release'
  local ID='' ID_LIKE='' platform
  . /etc/os-release
  case "$ID" in ubuntu|arch) echo "$ID"; return ;; esac
  for platform in ubuntu arch; do
    if [[ " $ID_LIKE " == *" $platform "* ]]; then echo "$platform"; return; fi
  done
  fail "Unsupported Linux distribution: $ID"
}

build() {
  local platform=$1 version=$2 stage package output
  [[ $platform == "$(detect_platform)" ]] || fail "Build $platform on its matching native distro"
  stage=$(mktemp -d)
  trap "rm -rf '$stage'" EXIT
  mkdir -p "$stage/source" artifacts
  # Build tracked source plus local edits, without generated output or dependencies.
  git ls-files -z | tar --null -T - -cf - | tar -xf - -C "$stage/source"
  (
    cd "$stage/source"
    python3 scripts/releaseVersion.py stamp "$version"
    cargo build --manifest-path matis-rust/Cargo.toml --target-dir matis-rust/target --release --locked --bins >&2
    [[ $(matis-rust/target/release/matis --version) == "matis $version" ]] || fail 'Binary version mismatch'
    cd dashboard
    bun install --no-save >&2
    bun run build >&2
  )
  package="$stage/root"
  mkdir -p "$package/usr/"{bin,lib/matis,share/applications,share/licenses/matis,share/pixmaps}
  cp "$stage/source/matis-rust/target/release/"{matis,matis-migrate} "$package/usr/bin/"
  cp -R "$stage/source/dashboard/.output" "$package/usr/lib/matis/dashboard"
  bun build scripts/desktop.ts --compile --outfile "$package/usr/bin/matis-ui" >&2
  cp LICENSE "$package/usr/share/licenses/matis/"
  cp dashboard/public/logo.png "$package/usr/share/pixmaps/matis.png"
  cp scripts/matis.desktop "$package/usr/share/applications/"
  if [[ $platform == ubuntu ]]; then
    mkdir -p "$package/DEBIAN"
    printf 'Package: matis\nVersion: %s\nArchitecture: %s\nMaintainer: Matis maintainers\nDepends: libc6, libgcc-s1, xdg-utils\nDescription: Matis CLI and desktop dashboard\n' "$version" "$(dpkg --print-architecture)" > "$package/DEBIAN/control"
    output="matis-$version-ubuntu.deb"
    dpkg-deb --root-owner-group --build "$package" "$stage/$output" >&2
  else
    cat > "$stage/PKGBUILD" <<PKGBUILD
pkgname=matis
pkgver=$version
pkgrel=1
pkgdesc='Matis CLI and desktop dashboard'
arch=('$(uname -m)')
license=('MIT')
depends=('glibc' 'gcc-libs' 'xdg-utils')
options=('!strip' '!debug')
package() { cp -a "\$startdir/root/usr" "\$pkgdir/"; }
PKGBUILD
    (cd "$stage" && PKGEXT=.pkg.tar.zst makepkg --nodeps --force >&2)
    output="matis-$version-arch.pkg.tar.zst"
    mv "$stage/matis-$version-1-$(uname -m).pkg.tar.zst" "$stage/$output"
  fi
  mv "$stage/$output" "artifacts/$output"
}

publish() {
  local version=$1 platform=$2 tag="v$1" file state expected actual commit
  [[ $platform == "$(detect_platform)" ]] || fail "Publish $platform on its matching native distro"
  case "$platform" in
    ubuntu) file="artifacts/matis-$version-ubuntu.deb" ;;
    arch) file="artifacts/matis-$version-arch.pkg.tar.zst" ;;
  esac
  [[ -s $file && ! -L $file ]] || fail "Missing or empty package: $file"
  expected=${file##*/}
  if state=$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null); then
    [[ $state == true ]] || fail "$tag is already published"
  else
    commit=$(git rev-parse --verify HEAD)
    gh release create "$tag" --draft --target "$commit" --title "$tag" --generate-notes
  fi
  gh release upload "$tag" "$file" --clobber
  actual=$(gh release view "$tag" --json assets --jq '.assets[].name' | LC_ALL=C sort)
  [[ $actual == "$expected" ]] || fail 'Remote assets differ; release remains draft'
  gh release edit "$tag" --draft=false
}

main() {
  local platform version
  platform=$(detect_platform)
  gh auth status
  git fetch origin --tags
  version=$(python3 scripts/releaseVersion.py)
  validate_version "$version"
  build "$platform" "$version"
  publish "$version" "$platform"
}

if [[ $# == 0 ]]; then
  main
  exit
fi

case $1 in
  --help|-h) help ;;
  version) [[ $# == 1 ]] || fail 'version takes no arguments'; python3 scripts/releaseVersion.py ;;
  build) [[ $# == 3 ]] || fail 'build requires platform and version'; validate_version "$3"; build "$2" "$3" ;;
  publish)
    [[ $# == 2 || $# == 3 ]] || fail 'publish requires version and optional platform'
    validate_version "$2"
    publish "$2" "${3:-$(detect_platform)}"
    ;;
  *) help >&2; exit 1 ;;
esac
