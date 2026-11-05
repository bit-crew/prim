# ~/.gitconfig, ported from rig/home/dot_gitconfig.tmpl. Identity comes from
# the flake's extraSpecialArgs: defaults ("devlab"/"devlab@localhost") or the
# real values passed by the prim wrapper at activation time (P3).
# Per-ORG identity (user + core.sshCommand bound to the org key) lives in
# ~/.gitconfig-<org> files that prim manages at runtime; the includeIf chain
# reaches them through orgs.config, so orgs never require a flake rebuild.
{ gitName, gitEmail, ... }:

{
  programs.git = {
    enable = true;
    includes = [ { path = "~/.config/git/orgs.config"; } ];
    settings = {
      user.name = gitName;
      user.email = gitEmail;
      init.defaultBranch = "main";
      core.editor = "hx";
    };
  };
}
