# Global packages (S2): what every machine needs, nothing more.
#
# Deliberately NOT here (per-repo flakes later, S3/S6e): toolchains
# (rust/cargo, node, python, go), infra CLIs (aws, gcloud, terraform, sops,
# wrangler), java, clangd, codelldb, lazysql.
#
# Installed via their programs.* modules instead of this list (same profile):
# helix, lazygit (editor.nix); starship, zoxide, direnv, nix-direnv (zsh.nix);
# atuin (editor.nix); zsh is the shell binary itself, listed here because
# programs.zsh only writes ~/.zshrc.
{ lib, pkgs, profile, ... }:

{
  home.packages = with pkgs; [
    # core
    git
    curl
    jq
    ripgrep
    fd
    fzf
    zsh
    croc # ad-hoc encrypted file transfer to/from the VPS

    # terminal niceties
    eza
    delta
    yazi
    bat # the zsh `cat` alias is global, so bat is too

    # herdr is in nixpkgs (same upstream, herdr.dev; 0.9.5 == local
    # self-install). PATH order keeps the self-installed ~/.local/bin copy
    # winning for now; this entry covers fresh machines.
    herdr

    # editor LSPs that are plain nixpkgs binaries (S2). clangd/codelldb stay
    # per-repo.
    marksman
    taplo
    basedpyright
    yaml-language-server
    rust-analyzer
  ] ++ lib.optionals (profile == "vps") [
    # mosh-server: roaming shell for clients on the tailnet. Only the VPS
    # serves mosh sessions; client installs are per device (see
    # rig/vps/mosh.md). SSH stays the auth path; mosh adds UDP 60000-60010
    # over tailscale0 only.
    mosh
  ];
}
