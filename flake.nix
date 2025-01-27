{
  description = "gcfeeder flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs?ref=nixpkgs-unstable";
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
            sha256 = "sha256-jtv1gCHstvA7Y4oQ++uy0uYHak4SsxgrfP2/5YxE+GQ=";
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
              (craneLib.fileset.commonCargoSources ./crates/gcinput)
              (craneLib.fileset.commonCargoSources ./crates/panic-log)
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
                exec = "gcfeeder %U";
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
          packages.gcfeeder = gcfeeder;

          devShells.default = craneLib.devShell {
            checks = self.checks.${system};

            packages = [ pkgs.taplo-cli ];

            env.LD_LIBRARY_PATH = lib.makeLibraryPath commonArgs.buildInputs;
          };
        };
    };
}
