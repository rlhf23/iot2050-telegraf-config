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
            cmake
            fontconfig
            nushell
            # System libraries
            clang
            gtk3
            libxkbcommon
            openssl
            xorg.libX11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            libGL
            xorg.libxcb
            xorg.libXfixes

            wayland
            libxkbcommon
          ];
          RUST_SRC_PATH = "${rustVersion}/lib/rustlib/src/rust/library";
          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.libGL}/lib:$LD_LIBRARY_PATH
            export LIBGL_DRIVERS_PATH=${pkgs.mesa.drivers}/lib/dri
            exec nu
          '';
        };
      }
    );
}
