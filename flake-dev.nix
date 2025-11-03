{
  description = "Minimal Rust development environment (fast)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain (native only, ~200MB)
            cargo
            rustc
            rust-analyzer
            
            # Build essentials (~50MB)
            pkg-config
            openssl
            
            # Optional: testing tools
            cargo-tarpaulin
            cargo-insta
          ];
          
          shellHook = ''
            echo "Minimal dev environment loaded (native builds only)"
            echo "For cross-compilation, use: nix develop"
          '';
        };
      }
    );
}
