{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  inherit (lib.modules) mkIf;
  inherit (lib.options) mkOption mkEnableOption mkPackageOption;
  inherit (lib) nameValuePair mapAttrs' mapAttrsToList foldl' types;

  tomlFormat = pkgs.formats.toml {};
  concatAttrs = attrList: foldl' (acc: attrs: acc // attrs) {} attrList;

  cfg = config.programs.metemplate;
in {
  options = {
    programs.metemplate = {
      enable = mkEnableOption "metemplate";

      package = mkPackageOption self.packages.${pkgs.stdenv.system} "metemplate" {
        default = "default";
      };

      projects = mkOption {
        type = types.attrsOf (types.submodule {
          options = {
            config = mkOption {
              inherit (tomlFormat) type;
              description = ''
                Project specific configuration defining templates and a possible values schema.
              '';
            };

            values = mkOption {
              type = types.attrsOf tomlFormat.type;
              default = {};
              description = ''
                Values definitions available in generation.
              '';
            };
            templates = mkOption {
              type = types.attrsOf types.lines;
              default = {};
              description = ''
                Raw templates that will get generated.
              '';
            };
          };
        });
        default = {};
        description = ''
          Projects to choose from in generation.
        '';
      };
    };
  };

  config = mkIf cfg.enable {
    home.packages = [cfg.package];

    xdg.configFile = concatAttrs (
      mapAttrsToList
      (
        projectName: project:
        # Project config
          {
            "metemplate/${projectName}/config.toml".source = tomlFormat.generate "metemplate-${projectName}-config" project.config;
          }
          # Values
          // (
            mapAttrs'
            (
              valuesName: values:
                nameValuePair "metemplate/${projectName}/values/${valuesName}.toml" {
                  source = tomlFormat.generate "metemplate-${projectName}-values-${valuesName}" values;
                }
            )
            project.values
          )
          # Templates
          // (
            mapAttrs'
            (
              templateName: template:
                nameValuePair "metemplate/${projectName}/templates/${templateName}" {
                  text = template;
                }
            )
            project.templates
          )
      )
      cfg.projects
    );
  };
}
