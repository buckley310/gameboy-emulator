{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };
  outputs =
    { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
      ];
    in
    {
      devShells = forAllSystems (system: {
        default =
          with nixpkgs.legacyPackages.${system};
          let
            libs = [
              cmake
              libGLX
              pulseaudio
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXinerama
              xorg.libXrandr
            ];
          in
          mkShell {
            buildInputs = libs;
            shellHook = ''
              export LIBCLANG_PATH='${llvmPackages.libclang.lib}/lib'
              export LD_LIBRARY_PATH='${lib.makeLibraryPath libs}'
            '';
          };
      });
    };
}
