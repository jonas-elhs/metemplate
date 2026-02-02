{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixos-unstable";

    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    fenix,
  }: let
    inherit (nixpkgs) lib;

    eachSystem = lib.genAttrs ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
  in {
    packages = eachSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      info = lib.importTOML ./Cargo.toml;

      inherit (fenix.packages.${system}.minimal) toolchain;
      rustPlatform = pkgs.makeRustPlatform {
        cargo = toolchain;
        rustc = toolchain;
      };
    in {
      metemplate = rustPlatform.buildRustPackage {
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

      default = self.packages.${system}.metemplate;
    });
  };
}
