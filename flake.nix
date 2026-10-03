{
  description = "lazy_json";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      git-hooks,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        testCmd =
          {
            profile,
          }:
          "${pkgs.cargo}/bin/cargo test --profile=${profile} --frozen --offline";

        check =
          { profile }:
          git-hooks.lib.${system}.run {
            src = ./.;

            settings = {
              rust = {
                check.cargoDeps = pkgs.rustPlatform.importCargoLock {
                  lockFile = ./Cargo.lock;
                  allowBuiltinFetchGit = true;
                };
                cargoManifestPath = "./Cargo.toml";
              };
            };

            hooks = {
              cargo-check = {
                enable = true;
              };

              cargo-test = {
                enable = true;
                entry = testCmd { inherit profile; };
                pass_filenames = false;
                stages = [ "pre-commit" ];
                verbose = true;
                files = "\\.rs$|Cargo\\.toml$|Cargo\\.lock$";
                excludes = [ "target/" ];
              };

              rustfmt = {
                enable = true;
                settings = {
                  check = true;
                  verbose = true;
                  config = {
                    max_width = 80;
                  };
                };
              };

              clippy = {
                enable = true;
                args = [ "-Dwarnings" ];
              };

              cargo-doc = {
                enable = true;
                entry = "cargo deadlinks";
                extraPackages = [
                  pkgs.cargo
                  pkgs.cargo-deadlinks
                ];
                pass_filenames = false;
                stages = [ "pre-commit" ];
                verbose = true;
                files = "\\.rs$|Cargo\\.toml$|Cargo\\.lock$";
                excludes = [ "target/" ];
              };

              cargo-sort = {
                enable = true;
                args = [
                  "--check"
                  "--no-format"
                ];
              };
            };
          };

        lazy_json =
          let
            lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
            version = "${builtins.substring 0 8 lastModifiedDate}-${self.shortRev or "dirty"}";
          in
          pkgs.rustPlatform.buildRustPackage rec {
            pname = "lazy_json";
            inherit version;

            src = pkgs.nix-gitignore.gitignoreSource [ ".gitignore" ] ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            profile = "release";

            doCheck = true;
            checkPhase = (check { inherit profile; }).shellHook;

            installPhase = ''
              mkdir -p $out
            '';
          };

        devShell = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            cargo-deadlinks
            lazygit
          ];
          shellHook = (check { profile = "dev"; }).shellHook;
        };
      in
      {
        checks = {
          pre-commit-check = check { profile = "release"; };
        };

        packages.default = lazy_json;
        devShells.default = devShell;
      }
    );
}
