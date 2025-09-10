{ config, lib, pkgs, ... }:

let
  cfg = config.services.menhue;
in
{
  ##### interface
  options.services.menhue = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether to enable menhue.";
    };

    package = lib.mkOption {
      type = lib.types.path;
      default = (pkgs.callPackage ./Cargo.nix { }).rootCrate.build;
      defaultText = "menhue.nix";
    };

    host = lib.mkOption {
      type = lib.types.str;
      default = "hue.lan";
    };

    username = lib.mkOption {
      type = lib.types.str;
      default = "";
    };
  };

  ##### implementation
  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];
    launchd.user.agents.menhue = {
      serviceConfig = {
        Label = "dk.marquart.menhue";
        ProgramArguments = [ "${cfg.package}/bin/menhue" ];
        RunAtLoad = true;
        EnvironmentVariables = {
          HOST = cfg.host;
          USERNAME_KEY = cfg.username;
          RUST_BACKTRACE = "1";
        };
      };
    };
  };
}
