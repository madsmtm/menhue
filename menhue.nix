{ rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "menhue";
  version = "0.1.0";

  src = builtins.path { path = ./.; name = "menhue"; };

  cargoLock = {
    lockFile = ./Cargo.lock;
    # outputHashes = {
    #   "objc2-0.6.0" = "...";
    # };
  };

  # For easier debugging
  # buildType = "debug";
}
