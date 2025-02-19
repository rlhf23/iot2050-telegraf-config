{
  description = "Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustVersion = pkgs.rust-bin.stable.latest.default;
      in {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustVersion
            rust-analyzer
            pkg-config
            # Add any other dependencies you need for egui
            cmake
            fontconfig
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            libGL
          ];
          RUST_SRC_PATH = "${rustVersion}/lib/rustlib/src/rust/library";
        };
      }
    );
}
