{ rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "menhue";
  version = "0.1.0";

  src = builtins.path { path = ./.; name = "menhue"; };

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "objc2-0.5.2" = "sha256-VCkZnZsR1o5Mo+HKPHnnaO46HZdCbAXxYllR7ShZ6l8=";
    };
  };

  # For easier debugging
  # buildType = "debug";
}
