let
  pkgs = import <nixpkgs> { config = { }; overlays = [ ]; };
  cargo_nix = pkgs.callPackage ./Cargo.nix { };
in
cargo_nix.rootCrate.build.override {
  runTests = true;
}
