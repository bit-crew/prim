{
  description = "prim global environment: home-manager profiles (vps, dc1)";

  inputs = {
    # S4: global software stays recent (nixos-unstable); flake.lock pins the
    # exact revision and updates are explicit via `nix flake update`.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # home-manager master is the branch that tracks nixos-unstable (no
    # release-26.11 branch exists yet); flake.lock pins the exact revision.
    home-manager = {
      url = "github:nix-community/home-manager/master";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, home-manager, ... }:
    let
      system = "x86_64-linux";

      # Guarded env read: pure evaluation (nix flake check, CI) gets the
      # fallback; activation with --impure picks up the real value.
      envOr = name: fallback:
        let v = builtins.getEnv name;
        in if v != "" then v else fallback;

      # Identity (S6d/S6e): sane defaults keep pure evaluation deterministic.
      # Real values arrive at activation time from the prim wrapper (P3) via
      # PRIM_GIT_NAME / PRIM_GIT_EMAIL (needs --impure at switch). Values are
      # identity, never secrets; no secrets enter the store.
      gitName = envOr "PRIM_GIT_NAME" "devlab";
      gitEmail = envOr "PRIM_GIT_EMAIL" "devlab@localhost";
      userName = envOr "USER" "debian";

      mkProfile = profile:
        home-manager.lib.homeManagerConfiguration {
          pkgs = nixpkgs.legacyPackages.${system};
          extraSpecialArgs = { inherit profile gitName gitEmail; };
          modules = [
            {
              home.username = userName;
              home.homeDirectory = "/home/${userName}";
              home.stateVersion = "26.05";
            }
            ./modules/tools.nix
            ./modules/zsh.nix
            ./modules/git.nix
            ./modules/editor.nix
          ];
        };
    in {
      homeConfigurations = {
        vps = mkProfile "vps";
        dc1 = mkProfile "dc1";
      };
    };
}
