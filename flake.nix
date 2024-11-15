{
  description = "flake for gcfeeder";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem = { system, ... }:
        let
          pkgs = import nixpkgs {
            inherit system;

            overlays = [ fenix.overlays.default ];
          };

          inherit (pkgs) lib;

          toolchain = fenix.packages.${system}.fromToolchainFile {
            file = ./rust-toolchain.toml;
            sha256 = "sha256-1uC3iVKIjZAtQ57qtpGIfvCPl1MTdTfWibjB37VWFPg=";
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
          src = craneLib.cleanCargoSource ./.;

          commonArgs = {
            inherit src;
            strictDeps = true;

            buildInputs = with pkgs; [
              libGL
              libxkbcommon
              vulkan-loader
              wayland
              xorg.libX11
              xorg.libXcursor
              xorg.libxcb
              xorg.libXi
            ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;

          individualCrateArgs = commonArgs // {
            inherit cargoArtifacts;
            inherit version;
          };

          fileSetForCrate = crate: lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              (craneLib.fileset.commonCargoSources ./lib/gcinput)
              (craneLib.fileset.commonCargoSources ./lib/panic-log)
              (craneLib.fileset.commonCargoSources ./crates/gcfeeder-core)
              (lib.fileset.maybeMissing ./crates/gcfeeder-core/resource)
              (craneLib.fileset.commonCargoSources crate)
              (lib.fileset.maybeMissing /${crate}/resource)
            ];
          };

          gcfeeder = craneLib.buildPackage (individualCrateArgs // rec {
            pname = "gcfeeder";
            cargoExtraArgs = "-p gcfeeder";
            src = fileSetForCrate ./crates/gcfeeder;

            nativeBuildInputs = with pkgs; [ makeWrapper ];

            postInstall = ''
              wrapProgram $out/bin/gcfeeder \
                --suffix LD_LIBRARY_PATH : ${lib.makeLibraryPath commonArgs.buildInputs}
            '';

            env.VERSION = "v${version}";

            desktopItems = with pkgs; [
              (makeDesktopItem {
                name = "gcfeeder";
                exec = "gcfeeder";
                comment = meta.description;
                desktopName = "gcfeeder";
                categories = [ "Utility" ];
              })
            ];

            # TODO: Derive from Cargo.toml?
            meta = with lib; {
              description = "A ViGEm / evdev feeder for GameCube controllers using the GameCube Controller Adapter.";
              mainProgram = "gcfeeder";
              homepage = "https://github.com/Sirius902/gcfeeder";
              platforms = platforms.linux;
            };
          });
        in
        with pkgs; {
          formatter = nixpkgs-fmt;

          checks = {
            inherit gcfeeder;

            gcfeeder-clippy = craneLib.cargoClippy (commonArgs // {
              inherit cargoArtifacts;
            });

            gcfeeder-fmt = craneLib.cargoFmt {
              inherit src;
            };
          };

          packages.default = gcfeeder;

          devShells.default = craneLib.devShell {
            checks = self.checks.${system};

            packages = [ ];

            env.LD_LIBRARY_PATH = lib.makeLibraryPath commonArgs.buildInputs;
          };
        };
    };
}
