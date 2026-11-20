{
  # prim — per-repo environment (S3/S6e: the toolchain lives WITH the repo).
  #
  # No version is pinned here: nixpkgs is tracked from nixos-unstable and no
  # flake.lock is committed, so every use resolves the current tree. The Rust
  # toolchain that used to live in ~/.cargo + ~/.rustup (rustup, 782M) now
  # comes from the store and is garbage collected like everything else.
  description = "prim repo environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          # toolchains this repo builds with
          rustc
          cargo
          rust-analyzer
          clippy
          rustfmt
        ];
      };
    };
}
