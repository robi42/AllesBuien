# `just` lists the recipes. They are shortcuts for people over what bin/ and pkgbuild/
# already hold, and nothing in the gate runs them: Factory and CI run bin/test directly.
# This file is protected all the same, because a person runs it, `just install` with sudo.

# Lists the recipes
default:
    @just --list

# Runs the gate, bin/test, as CI does: over twenty minutes from cold
ci:
    bin/test

# Runs bin/check-fast, the text-only gates of the pre-commit hook
check:
    bin/check-fast

# Runs the app from this checkout
run:
    cargo run --locked -p allesbuien

# The PKGBUILD's build() runs bin/build, so this is the whole release build.
#
# Builds the package with makepkg, without installing it
package:
    cd pkgbuild && makepkg -f

# The user's step, never an agent's: sudo asks for a password. makepkg's default options
# also build a debug package, which is left out. A Mode B install in ~/.local/bin comes
# first on PATH and would hide the package, so this refuses until README's Uninstall has
# removed it.
#
# Installs the package built by `just package`
install:
    @if [ -e "$HOME/.local/bin/allesbuien" ]; then \
        echo 'error: ~/.local/bin/allesbuien, a Mode B install, would hide the package; remove it as README's Uninstall section says' >&2; \
        exit 1; \
    fi
    sudo pacman -U "$(cd pkgbuild && makepkg --packagelist | grep -v '/allesbuien-debug-')"
