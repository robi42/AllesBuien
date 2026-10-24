# shellcheck shell=bash
#
# bin/test holds pkgbuild/THIRD-PARTY-LICENSES to Cargo.lock, and every one of
# its gates passes just the same once package() stops installing the file, so
# the install line is held here, where a commit that drops it is refused.

gate="shipped notices"
script="bin/check-fast"
outcome="red"
expect=("pkgbuild/PKGBUILD: package() no longer installs pkgbuild/THIRD-PARTY-LICENSES")

mutate() {
  sed -i '/^  install -Dm644 pkgbuild\/THIRD-PARTY-LICENSES /d' pkgbuild/PKGBUILD
}

# bin/check-gates reads these after sourcing the file.
: "$gate" "$script" "$outcome" "${expect[@]}"
