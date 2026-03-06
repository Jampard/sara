{
  description = "SARA — Solution Architecture Requirement for Alignment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    devenv = {
      url = "github:cachix/devenv";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs =
    {
      self,
      nixpkgs,
      devenv,
      rust-overlay,
      ...
    }@inputs:
    let
      inherit (nixpkgs) lib;

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = lib.genAttrs systems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "sara";
            version = "0.5.3";
            src = lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            env.OPENSSL_NO_VENDOR = "1";
            checkFlags = [ "--skip=repository::git::tests::test_is_git_repo" ];
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
        in
        {
          default = devenv.lib.mkShell {
            inherit inputs pkgs;
            modules = [
              (
                { pkgs, ... }:
                {
                  devenv.root =
                    let
                      devenvRoot = builtins.readFile ./.devenv-root;
                    in
                    pkgs.lib.mkIf (builtins.pathExists ./.devenv-root) (
                      pkgs.lib.strings.removeSuffix "\n" devenvRoot
                    );

                  packages = [
                    (pkgs.rust-bin.stable."1.93.0".default.override {
                      extensions = [
                        "rust-src"
                        "rust-analyzer"
                        "clippy"
                      ];
                    })
                    pkgs.pkg-config
                    pkgs.openssl
                  ];

                  env = {
                    RUST_BACKTRACE = "1";
                  };

                  enterShell = ''
                    export REPO_ROOT=$(git rev-parse --show-toplevel)
                  '';
                }
              )
            ];
          };
        }
      );
    };
}
