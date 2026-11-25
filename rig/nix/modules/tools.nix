# Global packages (S2): what every machine needs, nothing more.
#
# Toolchains (rust/cargo, node, python, go) and infra CLIs (aws, gcloud,
# terraform, sops, wrangler) live WITH their repo, declared in the repo's
# own flake.nix (see rig/nix/templates/repo/flake.nix and
# rig/common/entornos-repo.md). Each repo pins what it builds with, so
# prim builds with rust, gt with go and knt/ap with python + terraform.
#
# The VPS profile ALSO carries the toolchains it hosts, for one reason:
# activating a repo environment needs direnv (use flake in the repo's
# .envrc), which is per repo and opt-in. Until a repo opts in, its
# toolchain has to be reachable from a plain shell — otherwise
# `cargo build` in prim would break the moment rustup is gone. When a repo
# gets its .envrc wired, this can go back to being repo-only.
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

    # Python packaging for every repo that uses it: `uv` installs the venv
    # and the deps (`uv venv`, `uv pip install`), `ruff` lints and formats.
    # Both are small and universal, so they belong in the global profile
    # instead of a per-repo flake. The heavy uv tools (cxas-scrapi,
    # google-adk) are NOT here: they are not in nixpkgs, each repo installs
    # its own with `uv tool install` (see knt/ap's `make setup`).
    uv
    ruff

    # Toolchains of the repos this host builds. Their own flake.nix is the
    # precise declaration; these copies keep the shell working in repos
    # whose environment is not activated by direnv (see the header).
    # clippy and rustfmt come with them: `cargo clippy` and `cargo fmt` are
    # the checks prim's CI runs, so a shell without them is a shell that
    # cannot lint. bacon is the cargo watcher that was installed by hand.
    cargo
    rustc
    clippy
    rustfmt
    bacon
    go

    # Viewers for the data that lives here. ~/work is the SMB share, so the
    # CSVs and the markdown people drop from their phones land on this
    # host. The tools that read them belong here too: the render happens on
    # the VPS and the client only receives ANSI over SSH/Mosh, which is
    # why they work from any device on the tailnet.
    #   glow    - markdown renderer for the terminal
    #   miller  - CSV/TSV/JSON processor (mlr), with an interactive browser
    #   duckdb  - SQL over CSV/Parquet/JSON without importing anything
    # The DC-1 client installs its own copies (it also gets qsv, which is
    # not needed here); see rig/client/herramientas.md.
    glow
    miller
    duckdb

    # Images in the terminal. Yazi previews them natively when the OUTER
    # terminal speaks the kitty graphics protocol (herdr passes it through
    # and the client renders it: herdr's terminal.kitty_graphics is on by
    # default). chafa is the universal fallback: it draws with ANSI
    # truecolor, so it works in any terminal, over plain SSH, in a pipe and
    # on the phone. Helix never renders images (its picture support was
    # closed upstream), so this is the way to look at a PNG from the host.
    chafa
  ];
}
