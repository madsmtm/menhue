let
  pkgs = import <nixpkgs> { config = { }; overlays = [ ]; };
in
pkgs.callPackage ./menhue.nix { }
