{
  # prim — per-repo environment template (S3/S6e: deps live WITH the repo).
  # Copy to <repo>/flake.nix, uncomment what this project needs, then:
  #   nix flake update (generate/refresh flake.lock) && direnv allow
  description = "prim repo environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          # --- infra cloud (per-repo per S2; NOT global) ---
          # google-cloud-sdk
          # awscli2
          # terraform
          # sops
          # wrangler

          # --- toolchains this repo builds with ---
          # rustc cargo rust-analyzer
          # nodejs python3 openjdk
          # clang clang-tools codelldb

          # --- repo tooling ---
          # just
        ];
      };
    };
}
