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
        overlays = [ (import rust-overlay) ];

        pkgs = import nixpkgs {
          inherit system overlays;
        };

        linuxSystem =
          if system == "aarch64-darwin"
          then "aarch64-linux"
          else if system == "x86_64-darwin"
          then "x86_64-linux"
          else system;

        linuxPkgs = import nixpkgs {
          system = linuxSystem;
          inherit overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;
        hostXcrun = pkgs.writeShellScriptBin "xcrun" ''
          exec /usr/bin/xcrun "$@"
        '';

        cleanSrc = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            let
              baseName = builtins.baseNameOf path;
            in
              baseName != "target"
              && baseName != "node_modules"
              && baseName != ".git"
              && baseName != ".pnpm-store"
              && baseName != ".agents"
              && baseName != "dist";
        };

        frontendDist = linuxPkgs.buildNpmPackage {
          pname = "hurlbox-web";
          version = "0.1.0";
          src = ./web;
          npmDepsHash = linuxPkgs.lib.fakeHash;
          npmBuildScript = "build";
          installPhase = ''
            mkdir -p $out
            cp -r dist/* $out/
          '';
        };

        mkHurlbox = buildPkgs:
          buildPkgs.rustPlatform.buildRustPackage {
            pname = "hurlbox";
            version = "0.1.0";
            src = cleanSrc;
            cargoLock.lockFile = ./Cargo.lock;
            buildAndTestSubdir = "apps/server";

            nativeBuildInputs = [
              buildPkgs.pkg-config
              buildPkgs.clang
            ];

            buildInputs = [
              buildPkgs.openssl
              buildPkgs.libxml2
            ];

            OPENSSL_NO_VENDOR = 1;
            doCheck = false;

            preBuild = ''
              mkdir -p web/dist
              cp -r ${frontendDist}/* web/dist/
            '';
          };

        hurlbox = mkHurlbox pkgs;
        hurlboxLinux = mkHurlbox linuxPkgs;

        dockerImage = linuxPkgs.dockerTools.buildLayeredImage {
          name = "hurlbox";
          tag = "latest";

          contents = [
            hurlboxLinux
            linuxPkgs.openssl
            linuxPkgs.libxml2
            linuxPkgs.cacert
            linuxPkgs.iana-etc
          ];

          config = {
            Cmd = ["/bin/hurlbox"];
            Env = [
              "PORT=3030"
              "HOST=0.0.0.0"
            ];
            ExposedPorts = {
              "3030/tcp" = { };
            };
          };
        };
      in
      {
        packages.default = hurlbox;
        packages.aranshi = hurlbox;
        packages.docker = dockerImage;

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
