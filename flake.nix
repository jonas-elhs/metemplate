{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";

    systems.url = "github:nix-systems/default";
  };

  outputs = {
    self,
    nixpkgs,
    systems,
  }: let
    inherit (nixpkgs) lib;

    eachSystem = f:
      lib.genAttrs (import systems)
      (system: f nixpkgs.legacyPackages.${system});
  in {
    packages = eachSystem (pkgs: let
      info = lib.importTOML ./Cargo.toml;
    in {
      metemplate = pkgs.rustPlatform.buildRustPackage {
        pname = "metemplate";
        inherit (info.package) version;

        src = ./.;
        cargoLock = {lockFile = ./Cargo.lock;};

        meta = {
          inherit (info.package) description;
          homepage = "https://github.com/jonas-elhs/metemplate";
          license = lib.licenses.mit;
        };
      };

      default = self.packages.${pkgs.stdenv.system}.metemplate;
    });
  };
}
