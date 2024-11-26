{ rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "menhue";
  version = "0.1.0";

  src = builtins.path { path = ./.; name = "menhue"; };

  cargoLock = {
    lockFile = ./Cargo.lock;
    outputHashes = {
      "objc2-0.5.2" = "sha256-7UQ7Gl96vq7gHRgN2Io9yXUc1pxivTm+5lynGBJXxIE=";
    };
  };

  # For easier debugging
  # buildType = "debug";
}
