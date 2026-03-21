{
  description = "Aranshi";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;
        hostXcrun = pkgs.writeShellScriptBin "xcrun" ''
          exec /usr/bin/xcrun "$@"
        '';
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        aranshi = rustPlatform.buildRustPackage {
          pname = "aranshi";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.clang
          ];

          buildInputs = [
            pkgs.openssl
            pkgs.libxml2
          ];

          OPENSSL_NO_VENDOR = 1;
          DEVELOPER_DIR = "/Applications/Xcode.app/Contents/Developer";
          doCheck = false;
        };
      in
      {
        packages.default = aranshi;
        packages.aranshi = aranshi;

        devShells.default = pkgs.mkShell {
          packages = [
            hostXcrun
            rustToolchain
            pkgs.pkg-config
            pkgs.clang
            pkgs.openssl
            pkgs.libxml2
          ];

          shellHook = ''
            export PATH="${hostXcrun}/bin:$PATH"
            export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
            unset SDKROOT
          '';
        };
      }
    );
}
