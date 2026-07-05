{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachSystem [ "aarch64-linux" "x86_64-linux" ] (
      system:
      let
        overlays = [
          (import rust-overlay)
          (import ./llvm-mingw.nix)
        ];
        pkgs = import nixpkgs { inherit system overlays; };

        packages = with pkgs; [
          llvm-mingw
          (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
        ];

        buildInputs = with pkgs; [
          # macroquad
          pkg-config
          fontconfig
          libxkbcommon
          libGL
          libXcursor
          libXrandr
          libXi
          libX11
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          inherit packages buildInputs;
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };
      }
    );
}
