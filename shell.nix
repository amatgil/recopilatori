{
  pkgs ? import <nixpkgs> { },
  lib ? pkgs.lib,
}:
let
  packages = with pkgs; [
    rust-analyzer
    rustfmt
    clippy
    mold
    rust-bin.stable.latest.default
    sqlx-cli
    pkg-config
    libiconv openssl
    sqlite rlwrap
  ];
in
pkgs.mkShell {
  # Get dependencies from the main package
  inputsFrom = [ (pkgs.callPackage ./default.nix { }) ];
  nativeBuildInputs = packages;
  buildInputs = packages;
  env = {
    LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
    LD_LIBRARY_PATH = "${lib.makeLibraryPath packages}";
  };
}
