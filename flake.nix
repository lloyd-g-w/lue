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

      dioxusCli = pkgs.rustPlatform.buildRustPackage rec {
        pname = "dioxus-cli";
        version = "0.6.3";

        src = pkgs.fetchCrate {
          inherit pname version;
          hash = "sha256-wuIJq+UN1q5qYW4TXivq93C9kZiPHwBW5Ty2Vpik2oY=";
        };

        cargoHash = "sha256-L9r/nJj0Rz41mg952dOgKxbDS5u4zGEjSA3EhUHfGIk=";
        doCheck = false;
        nativeBuildInputs = [pkgs.pkg-config];
        buildInputs = [pkgs.openssl];
      };
    in {
      devShells.default = pkgs.mkShell {
        packages =
          [
            rustToolchain
            dioxusCli
            pkgs.binaryen
            pkgs.pkg-config
          ]
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

        shellHook = ''
          export HOME="$PWD/.nix-home"
          export XDG_CACHE_HOME="$HOME/.cache"
          export XDG_CONFIG_HOME="$HOME/.config"
          export XDG_DATA_HOME="$HOME/.local/share"
          export CARGO_BUILD_TARGET_DIR="$PWD/target"
          mkdir -p "$HOME" "$XDG_CACHE_HOME" "$XDG_CONFIG_HOME" "$XDG_DATA_HOME"
          echo "Rust toolchain: $(rustc --version)"
          echo "Dioxus CLI: $(dx --version)"
        '';
      };
    });
}
