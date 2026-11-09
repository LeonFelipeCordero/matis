#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

fail() { echo "$*" >&2; exit 1; }

help() {
  cat <<'HELP'
Usage: bash scripts/release.sh version
       bash scripts/release.sh build <ubuntu|arch|macos> <version>
       bash scripts/release.sh publish <version>

Local builds only, on the matching OS and native architecture (no cross-builds).
Use the same architecture for Ubuntu and Arch. Build Ubuntu on the oldest
supported Ubuntu release; binaries require that release's system libraries.
Requires Rust, Bun, Python 3.11+, Git; dpkg-deb on Ubuntu, makepkg on Arch,
and zip on macOS. No new application dependencies are installed by this script.
Dashboard dependencies are installed with bun install --no-save.
version uses existing stable tags and the Cargo baseline, without editing files.
build stamps Cargo files in a temporary source copy, not the working tree.
Outputs: artifacts/matis-<version>-ubuntu.deb, -arch.pkg.tar.zst, -macos.zip.
Linux installs matis, matis-migrate, matis-ui and an application launcher.
macOS zip contains Matis.app; CLI tools are in Contents/MacOS.
The native matis-ui executable serves the dashboard on loopback port 43127
and opens the default browser. It runs until stopped; port must be free.
Collect all three packages in artifacts/ on one machine before publish.
publish requires gh authentication and a pushed source commit. It creates or
resumes a draft v<version>, uploads packages, checks the exact remote asset set,
then publishes. It refuses changes to published releases. No Actions storage.
HELP
}

validate_version() {
  [[ $1 =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || fail 'Expected stable major.minor.patch version'
}

build() {
  local platform=$1 version=$2 host stage package output
  host=$(uname -s)
  case "$platform:$host" in
    ubuntu:Linux|arch:Linux)
      [[ -r /etc/os-release ]] || fail 'Missing /etc/os-release'
      . /etc/os-release
      [[ $ID == "$platform" || " ${ID_LIKE:-} " == *" $platform "* ]] || fail "Build $platform on $platform, not $ID"
      ;;
    macos:Darwin) ;;
    *) fail "Build $platform on its matching OS" ;;
  esac
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
  if [[ $platform == macos ]]; then
    package="$stage/Matis.app/Contents"
    mkdir -p "$package/MacOS" "$package/Resources"
    cp "$stage/source/matis-rust/target/release/"{matis,matis-migrate} "$package/MacOS/"
    cp -R "$stage/source/dashboard/.output" "$package/Resources/dashboard"
    cp LICENSE "$package/Resources/"
    bun build scripts/desktop.ts --compile --outfile "$package/MacOS/matis-ui" >&2
    cat > "$package/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>CFBundleExecutable</key><string>matis-ui</string><key>CFBundleIdentifier</key><string>dev.matis.desktop</string><key>CFBundleName</key><string>Matis</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>$version</string></dict></plist>
PLIST
    (cd "$stage" && zip -qr matis.zip Matis.app)
    output="matis-$version-macos.zip"
    mv "$stage/matis.zip" "artifacts/$output"
    return
  fi
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
  local version=$1 tag="v$1" file state expected actual commit
  local files=("artifacts/matis-$version-ubuntu.deb" "artifacts/matis-$version-arch.pkg.tar.zst" "artifacts/matis-$version-macos.zip")
  for file in "${files[@]}"; do [[ -s $file && ! -L $file ]] || fail "Missing or empty package: $file"; done
  expected=$(printf '%s\n' "${files[@]##*/}" | LC_ALL=C sort)
  actual=$(find artifacts -maxdepth 1 -name "matis-$version-*" -exec basename {} \; | LC_ALL=C sort)
  [[ $actual == "$expected" ]] || fail 'Expected exactly three platform packages for this version'
  if state=$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null); then
    [[ $state == true ]] || fail "$tag is already published"
  else
    commit=$(git rev-parse --verify HEAD)
    gh release create "$tag" --draft --target "$commit" --title "$tag" --generate-notes
  fi
  gh release upload "$tag" "${files[@]}" --clobber
  actual=$(gh release view "$tag" --json assets --jq '.assets[].name' | LC_ALL=C sort)
  [[ $actual == "$expected" ]] || fail 'Remote assets differ; release remains draft'
  gh release edit "$tag" --draft=false
}

case ${1:---help} in
  --help|-h) help ;;
  version) [[ $# == 1 ]] || fail 'version takes no arguments'; python3 scripts/releaseVersion.py ;;
  build) [[ $# == 3 ]] || fail 'build requires platform and version'; validate_version "$3"; build "$2" "$3" ;;
  publish) [[ $# == 2 ]] || fail 'publish requires version'; validate_version "$2"; publish "$2" ;;
  *) help >&2; exit 1 ;;
esac
