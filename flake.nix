{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    llvm-mingw-overlay.url = "github:Sharp0802/llvm-mingw-overlay";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      llvm-mingw-overlay,
      ...
    }:
    flake-utils.lib.eachSystem [ "aarch64-linux" "x86_64-linux" ] (
      system:
      let
        overlays = [
          rust-overlay.overlays.default
          llvm-mingw-overlay.overlays.default
        ];
        pkgs = import nixpkgs { inherit system overlays; };

        buildInputs = with pkgs; [
          pkg-config
          fontconfig
          wayland
          vulkan-loader
          vulkan-tools
          libxkbcommon
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          inherit buildInputs;

          packages = with pkgs; [
            cargo-deny
            llvm-mingw.latest.ucrt
            (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
          ];

          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };
      }
    );
}
