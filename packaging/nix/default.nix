# Nix package sketch (builds from source with the committed Cargo.lock).
{ lib, rustPlatform, fetchFromGitHub }:
rustPlatform.buildRustPackage rec {
  pname = "rhow";
  version = "0.1.0";
  src = fetchFromGitHub {
    owner = "line-19";
    repo = "rhow";
    rev = "v${version}";
    hash = lib.fakeHash;
  };
  cargoHash = lib.fakeHash;
  meta = with lib; {
    description = "Discover how to run and operate any repository";
    homepage = "https://github.com/line-19/rhow";
    license = licenses.mit;
    mainProgram = "rhow";
  };
}
