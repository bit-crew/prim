# ~/.zshrc, ported from rig/home/dot_zshrc.tmpl (chezmoi retired, S6d).
# Prompt hooks are owned by the programs.* modules below; home-manager places
# them at priority ~1000 inside initContent, i.e. between the early TERM guard
# (mkBefore) and the user block (mkOrder 1200) below.
{ profile, lib, ... }:

{
  programs.zsh = {
    enable = true;
    enableCompletion = true; # autoload -Uz compinit && compinit
    history = {
      size = 10000;
      save = 10000;
      path = "$HOME/.zsh_history";
      share = true; # SHARE_HISTORY
      ignoreDups = true; # HIST_IGNORE_DUPS
      ignoreSpace = true; # HIST_IGNORE_SPACE
    };
    shellAliases = {
      lg = "lazygit";
      ls = "eza";
      cat = "bat"; # bat itself is per-repo (not in the S2 global list)
      ".." = "cd ..";
      db = "lazysql"; # lazysql itself is per-repo (not in the S2 global list)
    };
    initContent = lib.mkMerge [
      (lib.mkBefore ''
      # Color del terminal: si el TERM que llega es pobre ("dumb"/vacio, p.ej. por una
      # sesion SSH mal negociada), forzar uno con color. Esto DEBE ir antes de starship
      # y demas: con un TERM pobre, los prompts consultan la paleta por OSC y las
      # respuestas (rgb:xxxx/...) se filtran como basura en pantalla. Con un TERM con
      # color + COLORTERM=truecolor, no consultan.
      case "$TERM" in
        ""|dumb|unknown) export TERM=xterm-256color ;;
      esac
      export COLORTERM=truecolor
      '')
      (lib.mkOrder 1200 (''
      # rustup / cargo (self-installed toolchains may still exist; harmless if absent)
      [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

      # Aterrizar en ~/work: al abrir una consola nueva o entrar por SSH sin estar
      # en ningun proyecto, la sesion arranca en la carpeta de organizaciones (una
      # por cuenta de git). La guarda (PRIM_LANDING exportada) evita re-mover shells
      # anidadas; si la sesion arranca en otra carpeta (pane, repo), no se toca.
      # Va antes del exec herdr para que las panes heredan ~/work como cwd.
      if [[ -o interactive ]] && [[ "$PWD" == "$HOME" ]] && [[ -d "$HOME/work" ]] && [[ -z "''${PRIM_LANDING:-}" ]]; then
        export PRIM_LANDING=1
        cd "$HOME/work"
      fi
    '' + lib.optionalString (profile == "vps") ''

      # Herdr como terminal por defecto del VPS. Toda sesion interactiva (Moshi, SSH,
      # consola) arranca o se adjunta a herdr. HERDR_ENV lo exporta herdr dentro de sus
      # panes; la guarda evita anidar herdr dentro de herdr. exec reemplaza zsh para que
      # al salir de herdr la sesion termine limpia.
      if [[ -o interactive ]] && [[ -z "$HERDR_ENV" ]] && command -v herdr >/dev/null; then
        exec herdr
      fi
      ''))
    ];
  };

  home.sessionPath = [
    # Same precedence as the chezmoi template: self-installed binaries win,
    # nix profile last as a fallback for fresh machines.
    "$HOME/.local/bin"
    "$HOME/.local/node/bin"
    "$HOME/.local/go/bin"
    "$HOME/.cargo/bin"
    "$HOME/go/bin"
    "$HOME/.nix-profile/bin"
  ];

  home.sessionVariables = {
    EDITOR = "hx";
    VISUAL = "hx";
    # HELIX_RUNTIME intentionally dropped: nixpkgs helix bundles its runtime
    # (share/helix/runtime) and locates it relative to the binary.
  };

  programs.starship.enable = true; # default config, as before
  programs.zoxide.enable = true;

  programs.direnv = {
    enable = true;
    enableZshIntegration = true;
    nix-direnv.enable = true; # S3: per-repo environments via `use flake`
  };
}
