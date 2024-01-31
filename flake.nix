{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, naersk, rust-overlay }:
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        naersk-lib = pkgs.callPackage naersk { };
        rust = pkgs.rust-bin.nightly.latest.default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        defaultPackage = naersk-lib.buildPackage {
          root = ./.;
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ];
        };
        devShell = with pkgs; mkShell {
          nativeBuildInputs = [ 
            cargo
            rust
            pkg-config
          ];
          buildInputs = [ rustfmt pre-commit rustPackages.clippy bacon openssl ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
      });
}
