{
  description = "Dev shell with Bun + Zsh from nixos-unstable";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      devShells.default = pkgs.mkShell {
        packages = [
          pkgs.bun
          pkgs.zsh
        ];

        # Optional: automatically switch to zsh when the shell is interactive
        shellHook = ''
          if [ -n "$PS1" ] && [ -z "$IN_NIX_SHELL_ZSH" ]; then
            export IN_NIX_SHELL_ZSH=1
            exec ${pkgs.zsh}/bin/zsh -l
          fi
        '';
      };
    });
}
