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
{ config, lib, profile, ... }:

{
  systemd.user.services.prim-inbox-cleanup = lib.optionalAttrs (profile == "vps") {
    unit = "${../../vps}/prim-inbox-cleanup.service";
  };

  systemd.user.timers.prim-inbox-cleanup = lib.optionalAttrs (profile == "vps") {
    unit = "${../../vps}/prim-inbox-cleanup.timer";
    # The raw timer carries its own [Install] section, but home-manager does
    # not read it: the symlink that arms the timer is declared here.
    wantedBy = [ "timers.target" ];
  };

  # The unit's ExecStart points at ~/.local/bin/prim-inbox-cleanup.
  home.file.".local/bin/prim-inbox-cleanup" = lib.optionalAttrs (profile == "vps") {
    source = "${../../vps}/prim-inbox-cleanup";
    executable = true;
  };
}
