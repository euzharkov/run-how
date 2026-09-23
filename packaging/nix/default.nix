# Nix package sketch (builds from source with the committed Cargo.lock).
{ lib, rustPlatform, fetchFromGitHub }:
rustPlatform.buildRustPackage rec {
  pname = "rhow";
  version = "0.1.0";
  src = fetchFromGitHub {
    owner = "euzharkov";
    repo = "run-how";
    rev = "v${version}";
    hash = lib.fakeHash;
  };
  cargoHash = lib.fakeHash;
  meta = with lib; {
    description = "Discover how to run and operate any repository";
    homepage = "https://github.com/euzharkov/run-how";
    license = licenses.mit;
    mainProgram = "rhow";
  };
}
