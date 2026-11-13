# Clipboard image grab for the DC-1 client (prim-clipboard, S6/S9).
#
# The Daylight DC-1 is an Android device running Termux: it is a CLIENT of
# the VPS, where the agents run. Its clipboard lives on the device, so an
# image has to be pushed to the VPS inbox (see systemd.nix) and its path
# travels back as text.
#
# The extra key below runs img-grab, which uploads the image and leaves the
# remote path in the clipboard as text: the next paste puts the path in the
# prompt and the agent reads the file.
#
# NOTE for the DC-1: Termux lives in /data/data/com.termux/files/home, not
# in /home/<user>. This module writes under the home-manager home
# directory, so activating the dc1 profile on the device needs that home
# directory to point at the Termux home. Until that is settled, apply the
# snippet from rig/client/imagenes.md by hand.
{ config, lib, pkgs, profile, ... }:

{
  # Profile-gated options use mkIf (not optionalAttrs): an empty attrset
  # still DECLARES the option, and home-manager then aborts with "the
  # option ... was accessed but has no value defined".
  home.file.".local/bin/img-grab" = lib.mkIf (profile == "dc1") {
    source = "${../../client}/img-grab";
    executable = true;
  };

  # Managed by the rig: a local edit is overwritten on the next activation.
  home.file.".termux/termux.properties" = lib.mkIf (profile == "dc1") {
    source = pkgs.writeText "termux.properties" ''
      # Managed by the rig (prim-clipboard).
      #
      # CTRL+ALT+G runs img-grab: it grabs the clipboard image, pushes it
      # to the VPS inbox and leaves the remote path in the clipboard as text.
      extra-keys = CTRL+ALT+G,~/.local/bin/img-grab\n
    '';
  };
}
