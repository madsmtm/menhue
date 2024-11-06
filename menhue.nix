{ rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "menhue";
  version = "0.1.0";

  src = builtins.path { path = ./.; name = "menhue"; };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  # For easier debugging
  # buildType = "debug";
}
