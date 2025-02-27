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

  outputs = {
    self,
    nixpkgs,
    crane,
    fenix,
    flake-parts,
    ...
  } @ inputs:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem = {system, ...}: let
        pkgs = import nixpkgs {
          inherit system;

          overlays = [fenix.overlays.default];
        };

        inherit (pkgs) lib;

        toolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-rqQlvQj2k8ohzPcGAr7kCsd2zkt033PaUbQWkNWWJd8=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = with pkgs; (lib.optionals stdenv.isLinux [
            # Required for tray-icon.
            pkg-config
          ]);

          buildInputs = with pkgs; (lib.optionals stdenv.isLinux [
            libGL
            libxkbcommon
            vulkan-loader
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libxcb
            xorg.libXi

            # Required for tray-icon.
            gdk-pixbuf
            glib
            gtk3
            libappindicator-gtk3
            xdotool
            zlib
          ]);
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;

        individualCrateArgs =
          commonArgs
          // {
            inherit cargoArtifacts;
            inherit version;
          };

        fileSetForCrate = crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              ./rules
              (craneLib.fileset.commonCargoSources ./crates/gcinput)
              (craneLib.fileset.commonCargoSources ./crates/panic-log)
              (craneLib.fileset.commonCargoSources ./crates/gcfeeder-core)
              (lib.fileset.maybeMissing ./crates/gcfeeder/resource)
              (lib.fileset.maybeMissing ./crates/gcfeeder-core/resource)
              (craneLib.fileset.commonCargoSources crate)
              (lib.fileset.maybeMissing /${crate}/resource)
            ];
          };

        gcfeeder = craneLib.buildPackage (individualCrateArgs
          // rec {
            pname = "gcfeeder";
            src = fileSetForCrate ./crates/gcfeeder;
            cargoExtraArgs = "-p gcfeeder --no-default-features";

            nativeBuildInputs =
              commonArgs.nativeBuildInputs
              ++ (with pkgs; [
                copyDesktopItems
                makeWrapper
              ]);

            postInstall = ''
              wrapProgram $out/bin/gcfeeder \
                --suffix LD_LIBRARY_PATH : ${lib.makeLibraryPath commonArgs.buildInputs}

              mkdir -p $out/lib/udev/rules.d
              cp rules/50-gcfeederd.rules $out/lib/udev/rules.d/

              install -Dm644 crates/gcfeeder/resource/icon.png $out/share/pixmaps/gcfeeder.png
            '';

            GCFEEDER_VERSION = "v${version}-${self.shortRev or self.dirtyShortRev}";

            desktopItems = with pkgs; [
              (makeDesktopItem {
                name = "gcfeeder";
                icon = "gcfeeder";
                exec = "gcfeeder %U";
                comment = meta.description;
                desktopName = "gcfeeder";
                categories = ["Utility"];
              })
            ];

            meta = with lib; {
              description = "A ViGEm / evdev feeder for GameCube controllers using the GameCube Controller Adapter.";
              longDescription = ''
                A ViGEm / evdev feeder for GameCube controllers using the GameCube Controller Adapter.

                Udev rules can be added as:

                  services.udev.packages = [ pkgs.gcfeeder ]
              '';
              homepage = "https://github.com/Sirius902/gcfeeder";
              # NOTE(Sirius902) No drivers are implemented for darwin. Putting this here mostly so it can be built for local dev.
              platforms = platforms.linux ++ platforms.darwin;
              mainProgram = "gcfeeder";
            };
          });
      in
        with pkgs; {
          formatter = alejandra;

          checks = {
            inherit gcfeeder;

            gcfeeder-clippy = craneLib.cargoClippy (commonArgs
              // {
                inherit cargoArtifacts;
              });

            gcfeeder-fmt = craneLib.cargoFmt {
              inherit src;
            };
          };

          apps.fmt = {
            type = "app";
            program = writeShellScriptBin "fmt" ''
              cargo fmt
              taplo fmt
              nix fmt
            '';
          };

          packages.default = gcfeeder;
          packages.gcfeeder = gcfeeder;

          devShells.default = craneLib.devShell {
            checks = self.checks.${system};

            packages = [
              pkgs.lldb
              pkgs.rust-analyzer-nightly
              pkgs.taplo-cli
              pkgs.tokio-console
              pkgs.just
            ];

            LD_LIBRARY_PATH = lib.makeLibraryPath commonArgs.buildInputs;
          };
        };
    };
}
