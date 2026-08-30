# AllesBuien

## Omarchy user installation

Install AllesBuien without root privileges from the repository root. The desktop entry, icon, and license use the same XDG data directory throughout:

```bash
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
cargo install --root ~/.local --path app
install -Dm644 pkgbuild/allesbuien.desktop "$DATA_HOME/applications/allesbuien.desktop"
install -Dm644 pkgbuild/allesbuien.svg "$DATA_HOME/icons/hicolor/scalable/apps/allesbuien.svg"
install -Dm644 LICENSE "$DATA_HOME/licenses/allesbuien/LICENSE"
update-desktop-database "$DATA_HOME/applications" || true
```

The Omarchy wiring in `~/.config/hypr/bindings.lua` binds **SUPER+ALT+W** to launch AllesBuien and applies a floating window rule for its observed app ID, `allesbuien`.

### Rollback

Restore the timestamped backup created during wiring, reload Hyprland, uninstall the Cargo binary, and remove the user payload:

```bash
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
cp ~/.config/hypr/bindings.lua.bak.<timestamp> ~/.config/hypr/bindings.lua
hyprctl reload
cargo uninstall --root ~/.local allesbuien
rm -f "$HOME/.local/bin/allesbuien" \
  "$DATA_HOME/applications/allesbuien.desktop" \
  "$DATA_HOME/icons/hicolor/scalable/apps/allesbuien.svg"
rm -rf "$DATA_HOME/licenses/allesbuien"
update-desktop-database "$DATA_HOME/applications" || true
```
