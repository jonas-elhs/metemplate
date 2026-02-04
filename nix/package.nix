{
  lib,
  rustPlatform,
}: let
  info = lib.importTOML ../Cargo.toml;
in
  rustPlatform.buildRustPackage {
    pname = "metemplate";
    inherit (info.package) version;

    src = ../.;
    cargoLock.lockFile = ../Cargo.lock;

    meta = {
      inherit (info.package) description;
      homepage = "https://github.com/jonas-elhs/metemplate";
      license = lib.licenses.mit;
      mainProgram = "metemplate";
    };
  }
