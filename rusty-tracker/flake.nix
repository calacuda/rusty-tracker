{
  inputs = {
    nixpkgs.url = "nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        libraries = with pkgs;[
          webkitgtk
          gtk3
          cairo
          gdk-pixbuf
          glib
          dbus
          openssl_3
          librsvg
          mesa
          libsoup
          alsa-lib
          libudev-zero
        ];

        packages = with pkgs; [
          curl
          wget
          pkg-config
          dbus
          openssl_3
          # glib
          # gtk3
          # webkitgtk
          # librsvg
          # gst_all_1.gstreamer
          # # Common plugins like "filesrc" to combine within e.g. gst-launch
          # gst_all_1.gst-plugins-base
          # # Specialized plugins separated by quality
          # gst_all_1.gst-plugins-good
          # gst_all_1.gst-plugins-bad
          # gst_all_1.gst-plugins-ugly
          # # Plugins to reuse ffmpeg to play almost every video format
          # gst_all_1.gst-libav
          # # Support the Video Audio (Hardware) Acceleration API
          # gst_all_1.gst-vaapi
          # mesa
          nodejs-slim
          nodePackages.npm
          websocat
          libudev-zero
          webkitgtk
          gtk3
          cairo
          gdk-pixbuf
          glib
          dbus
          openssl_3
          librsvg
          mesa
          libsoup
          alsa-lib
        ];
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = packages;

          shellHook =
            ''
              export XDG_DATA_DIRS=${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath libraries}:$LD_LIBRARY_PATH
            '';
        };
      });
}
