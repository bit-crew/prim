# Editor-adjacent configuration, ported from rig/home/dot_config/ (chezmoi
# retired, S6d): helix (config + languages + zenbones_warm theme), lazygit,
# atuin, herdr.
#
# languages.toml references tools that stay per-repo on purpose (S3): clangd,
# codelldb, python (debugpy from the project venv). Helix tolerates missing
# servers; they resolve once a repo's flake provides them.
{ profile, lib, ... }:

let
  # Warm charcoal palette — single source of truth for herdr / helix /
  # lazygit themes (was .chezmoidata.yaml; inlined here since chezmoi is
  # retired, S6d).
  palette = {
    bg = "#31312F"; # fondo: gris carbon calido (panel y sidebar iguales)
    bg_dim = "#31312F"; # igual que bg: sidebar sin diferencia visible
    bg_sel = "#3C3C39"; # seleccion / fila activa / linea actual
    fg = "#F6F1EA"; # texto crudo calido
    fg_dim = "#A8A39B"; # comentarios / texto sutil
    orange = "#B45A3C"; # acento principal: terracota
    yellow = "#C5A44D"; # amarillo ocre
    purple = "#9580B5"; # morado malva
    green = "#777C45"; # verde oliva
    blue = "#638A7D"; # azul-verdoso frio-calido
    cyan = "#91A99A"; # verde-gris claro (cyan de la paleta)
    red = "#8F4935"; # oxido: rojo calido de la paleta
  };
in
{
  programs.helix = {
    enable = true;
    settings = {
      theme = if profile == "vps" then "zenbones_warm" else "catppuccin_mocha";
      editor = {
        line-number = "relative";
        mouse = true;
        bufferline = "multiple";
        color-modes = true;
        auto-save = true;
        # Long lines wrap at the window edge, so they follow the terminal
        # size: widen or narrow the terminal and the wrap point moves.
        # In helix 25.01+ soft-wrap is a TABLE, not a boolean: writing
        # "soft-wrap = true" makes helix reject the whole config and fall
        # back to the defaults (no wrap at all).
        soft-wrap = {
          enable = true;
          max-wrap = 10;
          max-indent-retain = 40;
          wrap-indicator = "\u21ba ";
        };
        # Clipboard over SSH (OSC 52): y/p/R use the system-clipboard register
        # and termcode carries copies through the terminal to the client's
        # clipboard (helix >= 25.05/25.07; both keys are post-migration).
        default-yank-register = "+";
        clipboard-provider = "termcode";
        # Dotfiles visible everywhere: file picker (space+f) and the file
        # explorer (space+e — the option exists from helix 25.07, which the
        # nixpkgs helix provides; harmless on older versions).
        "file-picker".hidden = false;
        "file-explorer".hidden = false;
        cursor-shape = {
          insert = "bar";
          normal = "block";
          select = "underline";
        };
        indent-guides = {
          render = true;
          character = "▏";
          skip-levels = 1;
        };
        lsp = {
          display-messages = true;
          display-inlay-hints = true;
          auto-signature-help = true;
        };
        statusline = {
          left = [ "mode" "spinner" "file-name" "file-modification-indicator" ];
          center = [ ];
          right = [
            "diagnostics"
            "selections"
            "position"
            "file-encoding"
            "file-line-ending"
            "file-type"
          ];
          mode = {
            normal = "NORMAL";
            insert = "INSERT";
            select = "SELECT";
          };
        };
      };
      keys.normal = {
        # Recargar buffers desde disco (otro proceso toco archivos abiertos).
        C-r = ":reload-all";
        # lazygit integrado: Alt-g suspende Helix, abre lazygit y al volver
        # recarga (por si lazygit hizo checkout/stash). A-g, no C-g, para no
        # chocar con atajos del multiplexer (Herdr).
        A-g = [
          ":write-all"
          ":new"
          ":insert-output lazygit"
          ":buffer-close!"
          ":redraw"
          ":reload-all"
        ];
      };
    };

    # 1:1 port of dot_config/helix/languages.toml.tmpl. debugpy/codelldb are
    # debug adapters resolved from the project environment, not global.
    languages = {
      language-server = {
        basedpyright = {
          command = "basedpyright-langserver";
          args = [ "--stdio" ];
        };
        ruff = {
          command = "ruff";
          args = [ "server" ];
        };
        rust-analyzer.command = "rust-analyzer";
      } // lib.optionalAttrs (profile == "vps") {
        clangd.command = "clangd";
      };

      language = [
        {
          name = "python";
          scope = "source.python";
          injection-regex = "python";
          file-types = [ "py" ];
          language-servers = [ "basedpyright" "ruff" ];
          formatter = {
            command = "ruff";
            args = [ "format" "-" ];
          };
          auto-format = true;
          debugger = {
            name = "debugpy";
            transport = "stdio";
            command = "python";
            args = [ "-m" "debugpy.adapter" ];
            templates = [
              {
                name = "launch";
                request = "launch";
                completion = [ { name = "entrypoint"; completion = "filename"; } ];
                args = {
                  mode = "launch";
                  program = "{0}";
                  console = "integratedTerminal";
                };
              }
            ];
          };
        }
        {
          name = "toml";
          language-servers = [ "taplo" ];
        }
        {
          name = "yaml";
          language-servers = [ "yaml-language-server" ];
        }
        {
          name = "markdown";
          language-servers = [ "marksman" ];
        }
        {
          name = "rust";
          language-servers = [ "rust-analyzer" ];
          auto-format = true;
          debugger = {
            name = "codelldb";
            transport = "stdio";
            command = "codelldb";
            args = [ "--port" "{}" ];
            port-arg = "--port {}";
            templates = [
              {
                name = "binary";
                request = "launch";
                completion = [ { name = "binary"; completion = "filename"; } ];
                args = { program = "{0}"; };
              }
            ];
          };
        }
      ] ++ lib.optionals (profile == "vps") [
        {
          name = "c";
          language-servers = [ "clangd" ];
          auto-format = true;
        }
        {
          name = "cpp";
          language-servers = [ "clangd" ];
          auto-format = true;
        }
      ];
    };

    # zenbones "dark warm" (base zenbones_dark, MIT): contrast over color.
    # vps only; dc1 uses the bundled catppuccin_mocha.
    themes = lib.optionalAttrs (profile == "vps") {
      zenbones_warm = {
        palette = palette;
        "ui.background" = { fg = "fg"; bg = "bg"; };
        "ui.text" = { fg = "fg"; };
        "ui.text.focus" = { fg = "fg"; modifiers = [ "bold" ]; };
        "ui.menu" = { fg = "fg"; bg = "bg_dim"; };
        "ui.menu.selected" = { fg = "fg"; bg = "bg_sel"; modifiers = [ "bold" ]; };
        "ui.popup" = { fg = "fg"; bg = "bg_dim"; };
        "ui.help" = { fg = "fg"; bg = "bg_dim"; };
        "ui.window" = { fg = "bg_sel"; };
        "ui.linenr" = { fg = "fg_dim"; };
        "ui.linenr.selected" = { fg = "orange"; modifiers = [ "bold" ]; };
        "ui.gutter" = { bg = "bg"; };
        "ui.cursor" = { fg = "bg"; bg = "fg"; };
        "ui.cursor.primary" = { fg = "bg"; bg = "orange"; };
        "ui.cursor.match" = { fg = "yellow"; bg = "bg_sel"; modifiers = [ "bold" ]; };
        "ui.selection" = { bg = "bg_sel"; };
        "ui.selection.primary" = { bg = "bg_sel"; };
        "ui.statusline" = { fg = "fg"; bg = "bg_dim"; };
        "ui.statusline.inactive" = { fg = "fg_dim"; bg = "bg_dim"; };
        "ui.statusline.normal" = { fg = "bg"; bg = "fg"; modifiers = [ "bold" ]; };
        "ui.statusline.insert" = { fg = "bg"; bg = "green"; modifiers = [ "bold" ]; };
        "ui.statusline.select" = { fg = "bg"; bg = "orange"; modifiers = [ "bold" ]; };
        "ui.virtual.ruler" = { bg = "bg_dim"; };
        "ui.virtual.whitespace" = { fg = "bg_sel"; };
        "ui.virtual.indent-guide" = { fg = "bg_sel"; };
        "ui.virtual.inlay-hint" = { fg = "fg_dim"; };
        "comment" = { fg = "fg_dim"; modifiers = [ "italic" ]; };
        "variable" = { fg = "fg"; };
        "variable.builtin" = { fg = "purple"; };
        "variable.parameter" = { fg = "fg"; };
        "variable.other.member" = { fg = "fg"; };
        "constant" = { fg = "orange"; };
        "constant.numeric" = { fg = "orange"; };
        "constant.character" = { fg = "green"; };
        "constant.character.escape" = { fg = "cyan"; };
        "string" = { fg = "green"; };
        "type" = { fg = "yellow"; };
        "type.builtin" = { fg = "yellow"; };
        "constructor" = { fg = "yellow"; };
        "function" = { fg = "fg"; modifiers = [ "bold" ]; };
        "function.builtin" = { fg = "fg"; modifiers = [ "bold" ]; };
        "function.macro" = { fg = "orange"; };
        "keyword" = { fg = "purple"; };
        "keyword.control" = { fg = "purple"; };
        "keyword.operator" = { fg = "fg"; };
        "operator" = { fg = "fg"; };
        "punctuation" = { fg = "fg_dim"; };
        "punctuation.delimiter" = { fg = "fg_dim"; };
        "label" = { fg = "orange"; };
        "tag" = { fg = "blue"; };
        "namespace" = { fg = "yellow"; };
        "special" = { fg = "cyan"; };
        "markup.heading" = { fg = "orange"; modifiers = [ "bold" ]; };
        "markup.bold" = { modifiers = [ "bold" ]; };
        "markup.italic" = { modifiers = [ "italic" ]; };
        "markup.link.url" = { fg = "blue"; modifiers = [ "underlined" ]; };
        "markup.link.text" = { fg = "orange"; };
        "markup.raw" = { fg = "green"; };
        "markup.list" = { fg = "purple"; };
        "diagnostic.error" = { underline = { color = "red"; style = "curl"; }; };
        "diagnostic.warning" = { underline = { color = "yellow"; style = "curl"; }; };
        "diagnostic.info" = { underline = { color = "blue"; style = "curl"; }; };
        "diagnostic.hint" = { underline = { color = "cyan"; style = "curl"; }; };
        "error" = { fg = "red"; };
        "warning" = { fg = "yellow"; };
        "info" = { fg = "blue"; };
        "hint" = { fg = "cyan"; };
        "diff.plus" = { fg = "green"; };
        "diff.delta" = { fg = "yellow"; };
        "diff.minus" = { fg = "red"; };
      };
    };
  };

  programs.lazygit = {
    enable = true;
    settings = {
      gui = {
        # vps: theme from the shared palette (consistent with herdr/helix).
        theme =
          if profile == "vps" then {
            activeBorderColor = [ palette.orange "bold" ];
            inactiveBorderColor = [ palette.fg_dim ];
            selectedLineBgColor = [ palette.bg_sel ];
            optionsTextColor = [ palette.blue ];
          } else {
            activeBorderColor = [ "green" "bold" ];
            inactiveBorderColor = [ "white" ];
            selectedLineBgColor = [ "reverse" ];
          };
        nerdFontsVersion = "3";
        commitLength.show = true;
      };
      git = {
        paging = {
          colorArg = "always";
          pager = "delta --dark --paging=never --side-by-side --line-numbers";
        };
        skipHookPrefix = "WIP";
      };
      update = {
        method = "never";
        days = 14;
      };
      confirmOnQuit = false;
      os = {
        editPreset = "helix";
        disableStartupPopups = true;
      };
    };
  };

  programs.atuin = {
    enable = true;
    enableZshIntegration = true;
    settings = {
      inline_height = 15;
      style = "compact";
      show_tabs = false;
      enter_accept = true;
      auto_sync = false;
      update_check = false;
    };
  };

  # herdr has no home-manager module; plain file generation, vps only.
  xdg.configFile = lib.optionalAttrs (profile == "vps") {
    "herdr/config.toml".text = ''
      # ~/.config/herdr/config.toml — theme: warm charcoal (shared palette).

      # Ya elegiste el modo: no volver a mostrar el onboarding.
      onboarding = false

      [theme]
      # Base "terminal": herdr no impone un fondo propio, asi que panel_bg (abajo)
      # controla el fondo del panel. Evita que un tema base oscuro gane sobre la paleta.
      name = "terminal"

      # Override de tokens sobre la base, desde la paleta central.
      [theme.custom]
      panel_bg = "${palette.bg}"
      sidebar_bg = "${palette.bg_dim}"
      active_row_bg = "${palette.bg_sel}"
      selection_bg = "${palette.bg_sel}"
      text = "${palette.fg}"
      accent = "${palette.orange}"
      yellow = "${palette.yellow}"
      red = "${palette.red}"
      green = "${palette.green}"
      blue = "${palette.blue}"

      [ui]
      # Pestanas: nombre automatico segun el proceso (no preguntar al crear).
      prompt_new_tab_name = false
      # Estado de agentes con glifos distintos por estado (mas legible que puntos).
      status_indicators = "symbols"
      # Espaciado entre paneles para una apariencia limpia.
      pane_gaps = true

      # Sidebar de agentes: ver de un vistazo QUE hace cada agente, no solo el modelo.
      # terminal_title_stripped = titulo del proceso (la tarea), sin el spinner.
      [ui.sidebar.agents]
      row_gap = 0
      rows = [["state_icon", "workspace", "tab"], ["terminal_title_stripped"]]

      # Spaces compactos: workspace + rama + estado git en UNA sola linea.
      [ui.sidebar.spaces]
      row_gap = 0
      rows = [["state_icon", "workspace", "branch", "git_status"]]

      [ui.toast]
      delivery = "herdr"

      # ── Navegacion ─────────────────────────────────────────────────────────────
      # Modelo: hjkl SIEMPRE es el movimiento. El CONTEXTO define sobre que te mueves.
      #
      # navigate mode (entrar con prefix+g = "goto"): hjkl puro, sin modificadores.
      #   h / l  -> moverse entre PANELES (terminales) izquierda / derecha
      #   j / k  -> moverse entre SPACES (workspaces) abajo / arriba
      # Fuera del modo, prefix+hjkl sigue moviendo paneles (atajo directo).
      [keys]
      goto = "prefix+g"
      # Paneles: solo horizontal en navigate mode (h/l); vertical queda para spaces.
      navigate_pane_left = "h"
      navigate_pane_right = "l"
      # Spaces (workspaces): vertical con j/k en navigate mode.
      navigate_workspace_down = "j"
      navigate_workspace_up = "k"

      # Agentes: herdr no tiene navigate-mode para agentes, asi que van por prefix.
      # Letras simples (robustas por SSH/movil; alt/punctuation se pierden).
      next_agent = "prefix+i"
      previous_agent = "prefix+u"

      # Atajos para abrir herramientas en popup (prefix = ctrl+b, luego la tecla).
      # popup = terminal modal que se cierra al salir del programa, sin tocar el layout.
      [[keys.command]]
      key = "prefix+plus"
      type = "popup"
      command = "lazygit"
      width = "90%"
      height = "90%"

      [[keys.command]]
      key = "prefix+a"
      type = "popup"
      command = "hx ."
      width = "90%"
      height = "90%"

      [[keys.command]]
      key = "prefix+d"
      type = "popup"
      command = "lazysql"
      width = "90%"
      height = "90%"
    '';
  };
}
