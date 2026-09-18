# shellcheck shell=bash
#
# pkgver= is the one version surface bin/install reads, and the one nothing at
# tag time compares: release.yml checks three Cargo manifests, so a PKGBUILD
# left behind builds a package named after the previous release, and the
# README's download line names a file that was never attached.

gate="release version"
script="bin/check-fast"
outcome="red"
expect=("pkgbuild/PKGBUILD" "9.9.9")

mutate() {
  awk '/^pkgver=/ { $0 = "pkgver=9.9.9" } { print }' pkgbuild/PKGBUILD >pkgbuild/PKGBUILD.violation
  mv pkgbuild/PKGBUILD.violation pkgbuild/PKGBUILD
  grep -qx 'pkgver=9.9.9' pkgbuild/PKGBUILD
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
