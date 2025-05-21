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
          # crossSystem = {
          #   config = "x86_64-w64-mingw32";
          # };
        };
        rustVersion = pkgs.pkgsBuildHost.rust-bin.stable.latest.default.override {
          targets = ["x86_64-pc-windows-gnu"];
        };
        libPath = with pkgs;
          lib.makeLibraryPath [
            libGL
            libxkbcommon
            wayland
          ];
      in {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustVersion
            rustup
            rust-analyzer
            pkg-config
            zlib
            openssl
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
            # windows
            # windows.mingw_w64_pthreads
            wine64
          ];
          RUST_SRC_PATH = "${rustVersion}/lib/rustlib/src/rust/library";
          # RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
          LD_LIBRARY_PATH = libPath;
          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.libGL}/lib:$LD_LIBRARY_PATH
            export LIBGL_DRIVERS_PATH=${pkgs.mesa.drivers}/lib/dri
            exec nu
          '';
        };
      }
    );
}
