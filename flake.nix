{
  description = "json";

  inputs = {
    nixpkgs.url = "nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    import-cargo.url = "github:edolstra/import-cargo";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      import-cargo,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        json =
          let
            lastModifiedDate = self.lastModifiedDate or self.lastModified or "19700101";
            version = "${builtins.substring 0 8 lastModifiedDate}-${self.shortRev or "dirty"}";
          in
          {
            inShell ? false,
          }:
          pkgs.stdenv.mkDerivation rec {
            name = "json-${version}";

            src = if inShell then null else pkgs.nix-gitignore.gitignoreSource [ ".gitignore" ] ./.;

            buildInputs =
              with pkgs;
              [
                cargo
              ]
              ++ (
                if inShell then
                  [
                    lazygit
                    perf
                  ]
                else
                  [
                    (import-cargo.builders.importCargo {
                      lockFile = ./Cargo.lock;
                      inherit pkgs;
                    }).cargoHome
                  ]
              );

            target = "--release";
            doCheck = true;

            checkPhase = "cargo test ${target} --frozen --offline";
            installPhase = ''
              mkdir -p $out
            '';
          };
      in
      {
        packages.default = json { };
        devShells.default = json { inShell = true; };
      }
    );
}
