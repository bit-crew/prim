# Ephemeral image inbox cleanup (prim-clipboard, S5/S7/S8).
#
# The inbox lives in the XDG cache dir (~/.cache/prim/inbox) and holds the
# images the clients pushed from their clipboard. It is disposable by
# construction: outside ~/work, so a screenshot never reaches a repo, a git
# index or the SMB share.
#
# Policy (the user's, S2): the timer runs TWICE A DAY and deletes ONLY what
# is older than 12 hours, so the worst case retention is ~24h.
#
# The unit files and the script are raw files under rig/vps/: one source
# of truth. They are imported here and are also installable by hand (which
# is how the cleanup landed on the VPS before Nix was in place).
#
# The units are placed as plain files (home.file), NOT through home-manager's
# systemd module: that module renders a unit from a structured attrset and
# cannot import a raw unit file, so using it would mean a second copy of the
# unit content. The raw files stay the only source; the timer is armed by an
# activation snippet that creates the same symlink systemd's "enable" makes.
{ lib, profile, ... }:

{
  # Profile-gated options use mkIf (not optionalAttrs): an empty attrset
  # still DECLARES the option, and home-manager then aborts with "the
  # option ... was accessed but has no value defined".
  home.file.".config/systemd/user/prim-inbox-cleanup.service" = lib.mkIf (profile == "vps") {
    source = "${../../vps}/prim-inbox-cleanup.service";
  };

  home.file.".config/systemd/user/prim-inbox-cleanup.timer" = lib.mkIf (profile == "vps") {
    source = "${../../vps}/prim-inbox-cleanup.timer";
  };

  # The unit's ExecStart points at ~/.local/bin/prim-inbox-cleanup.
  home.file.".local/bin/prim-inbox-cleanup" = lib.mkIf (profile == "vps") {
    source = "${../../vps}/prim-inbox-cleanup";
    executable = true;
  };

  # Arm the timer: the symlink systemd's "enable" creates (the raw timer
  # carries its own [Install] section, which home-manager does not read).
  # home.activation entries are DAG nodes: { data = "<script>"; }.
  home.activation.primInboxCleanupTimer = lib.mkIf (profile == "vps") {
    data = ''
      mkdir -p "$HOME/.config/systemd/user/timers.target.wants"
      ln -sfn ../prim-inbox-cleanup.timer "$HOME/.config/systemd/user/timers.target.wants/prim-inbox-cleanup.timer"
    '';
    after = [ "installPackages" ];
    before = [ ];
  };
}
