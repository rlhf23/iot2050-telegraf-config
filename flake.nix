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
          targets = ["x86_64-pc-windows-gnu" "aarch64-unknown-linux-gnu" "aarch64-unknown-linux-musl"];
        };
        aarch64Openssl = pkgs.pkgsCross.aarch64-multiplatform.openssl;
        libPath = with pkgs;
          lib.makeLibraryPath [
            libGL
            libxkbcommon
          ];
      in {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustVersion
            rust-analyzer
            cargo-machete
            cargo-insta
            cargo-tarpaulin
            cargo-cross
            pkg-config
            zlib
            openssl
            cmake
            fontconfig
            nushell
            # Cross-compilation toolchain
            pkgsCross.aarch64-multiplatform.stdenv.cc
            pkgsCross.aarch64-multiplatform.buildPackages.gcc
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
          # Cross-compilation environment variables
          AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_DIR = "${aarch64Openssl.dev}";
          AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR = "${aarch64Openssl.out}/lib";
          AARCH64_UNKNOWN_LINUX_GNU_OPENSSL_INCLUDE_DIR = "${aarch64Openssl.dev}/include";
          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.libGL}/lib:$LD_LIBRARY_PATH
            export LIBGL_DRIVERS_PATH=${pkgs.mesa.drivers}/lib/dri
            exec nu
          '';
        };
      }
    );
}
