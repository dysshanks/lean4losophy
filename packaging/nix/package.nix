# SPDX-License-Identifier: Apache-2.0
# Nix package for lean4losophy.
#
# Status: skeleton for Gate G1. It builds and tests the Rust workspace today.
# When crates/lean4losophy lands in M1, add its GTK dependencies and the desktop
# entry to the install phase. `lean4` is never a build input: Lean is an
# optional runtime download (ADR-0003).
{
  lib,
  rustPlatform,
  gtk4,
  libadwaita,
  pkg-config,
  wrapGAppsHook4,
}:

rustPlatform.buildRustPackage {
  pname = "lean4losophy";
  version = "0.0.0";

  src = ../..;

  cargoLock.lockFile = ../../Cargo.lock;

  cargoBuildFlags = [ "--workspace" ];
  cargoTestFlags = [ "--workspace" ];

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
  ];

  buildInputs = [
    gtk4
    libadwaita
  ];

  postInstall = ''
    install -Dm644 ${../flatpak/io.github.dysshanks.lean4losophy.desktop} \
      $out/share/applications/io.github.dysshanks.lean4losophy.desktop
    install -Dm644 ${../flatpak/io.github.dysshanks.lean4losophy.metainfo.xml} \
      $out/share/metainfo/io.github.dysshanks.lean4losophy.metainfo.xml
  '';

  meta = {
    description = "Local-first notebook for structured philosophical arguments";
    longDescription = ''
      lean4losophy stores notes as plain Markdown in a folder you own. Arguments can
      be written as structured premises and conclusions and optionally checked
      with Lean 4, which is downloaded separately and never bundled.
    '';
    homepage = "https://github.com/dysshanks/lean4losophy";
    license = lib.licenses.asl20;
    mainProgram = "lean4losophy";
    platforms = lib.platforms.linux;
  };
}
