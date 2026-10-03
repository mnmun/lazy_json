{
  description = "lazy_json";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    import-cargo.url = "github:edolstra/import-cargo";
    git-hooks.url = "github:cachix/git-hooks.nix";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      import-cargo,
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
          {
            inShell ? false,
          }:
          pkgs.stdenv.mkDerivation rec {
            name = "lazy_json-${version}";

            src = if inShell then null else pkgs.nix-gitignore.gitignoreSource [ ".gitignore" ] ./.;

            buildInputs =
              with pkgs;
              [
                cargo
                cargo-deadlinks
              ]
              ++ (
                if inShell then
                  [
                    lazygit
                  ]
                else
                  [
                    (import-cargo.builders.importCargo {
                      lockFile = ./Cargo.lock;
                      inherit pkgs;
                    }).cargoHome
                  ]
              );

            profile = if inShell then "dev" else "release";

            doCheck = true;

            checkPhase = (check { inherit profile; }).shellHook;

            installPhase = ''
              mkdir -p $out
            '';

            shellHook = if inShell then (check { inherit profile; }).shellHook else "";
          };
      in
      {
        checks = {
          pre-commit-check = check { profile = "release"; };
        };

        packages.default = lazy_json { };
        devShells.default = lazy_json { inShell = true; };
      }
    );
}
