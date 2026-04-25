{
  description = "Rust + Dioxus queue app development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      overlays = [(import rust-overlay)];
      pkgs = import nixpkgs {
        inherit system overlays;
      };

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = ["wasm32-unknown-unknown"];
      };
    in {
      devShells.default = pkgs.mkShell {
        packages =
          [
            rustToolchain
            pkgs.dioxus-cli
            pkgs.binaryen
            pkgs.pkg-config
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

        shellHook = ''
          export CARGO_BUILD_TARGET_DIR="${toString ./.}/target"
          echo "Rust toolchain: $(rustc --version)"
          echo "Dioxus CLI: $(dx --version)"
        '';
      };
    });
}
