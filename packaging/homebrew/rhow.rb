# Homebrew formula template. The release workflow's SHA256SUMS provides the hashes;
# publish this in a `homebrew-tap` repository as Formula/rhow.rb.
class Rhow < Formula
  desc "Discover how to run and operate any repository"
  homepage "https://github.com/line-19/rhow"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/line-19/rhow/releases/download/v#{version}/rhow-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
    on_intel do
      url "https://github.com/line-19/rhow/releases/download/v#{version}/rhow-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/line-19/rhow/releases/download/v#{version}/rhow-#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
    on_intel do
      url "https://github.com/line-19/rhow/releases/download/v#{version}/rhow-#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "REPLACE_WITH_SHA256"
    end
  end

  def install
    bin.install "rhow"
  end

  test do
    assert_match "Development", shell_output("cd #{testpath} && echo '{\"scripts\":{\"dev\":\"vite\"}}' > package.json && #{bin}/rhow --no-runtime")
  end
end
