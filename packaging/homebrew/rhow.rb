# Homebrew formula template. The release workflow's SHA256SUMS provides the hashes;
# publish this in a `homebrew-tap` repository as Formula/rhow.rb.
class Rhow < Formula
  desc "Discover how to run and operate any repository"
  homepage "https://github.com/euzharkov/run-how"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/euzharkov/run-how/releases/download/v#{version}/rhow-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "ea60cdea4c265c9f7ace4f829bc1ca9634d269a7c3a005262a9929fc1d937cc8"
    end
    on_intel do
      url "https://github.com/euzharkov/run-how/releases/download/v#{version}/rhow-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "ac4d75b2d2bb07cf5da94ffeed7d9e42dc0b60c8c4da754da0085a1ee4db2052"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/euzharkov/run-how/releases/download/v#{version}/rhow-#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "e8de36faf337997cdcb0325f40e6867b3f3ee4ebe213acf3c917b3a9e51b6131"
    end
    on_intel do
      url "https://github.com/euzharkov/run-how/releases/download/v#{version}/rhow-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "13dcbba369140faad44ec8dd59d2062ba592ccf0943fad1e4e600b56f89b7ff0"
    end
  end

  def install
    bin.install "rhow"
  end

  test do
    assert_match "npm run dev", shell_output("cd #{testpath} && echo '{\"scripts\":{\"dev\":\"vite\"}}' > package.json && #{bin}/rhow --no-runtime")
  end
end
