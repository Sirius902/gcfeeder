{
  description = "flake for gcfeeder";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=24.05";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { inherit system; };
    in
    with pkgs; rec {
      formatter = pkgs.nixpkgs-fmt;

      devShell = mkShell rec {
        buildInputs = [
          # necessary for building wgpu in 3rd party packages (in most cases)
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          alsa-lib
          fontconfig
          freetype
          shaderc
          directx-shader-compiler
          pkg-config
          cmake
          mold # could use any linker, needed for rustix (but mold is fast)

          libGL
          vulkan-headers
          vulkan-loader
          vulkan-tools
          vulkan-tools-lunarg
          vulkan-extension-layer
          vulkan-validation-layers # don't need them *strictly* but immensely helpful

          # necessary for developing (all of) wgpu itself
          cargo-nextest
          cargo-fuzz

          # nice for developing wgpu itself
          typos

          # if you don't already have rust installed through other means,
          # this shell.nix can do that for you with this below
          yq # for tomlq below
          rustup

          # nice tools
          gdb
          rr
          evcxr
          valgrind
          renderdoc
        ];

        LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
      };
    });
}
