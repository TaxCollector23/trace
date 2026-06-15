# TraceGuard Homebrew formula.
#
# Installs the `trg` binary from GitHub Releases and symlinks `traceguard` to it.
# The url/sha256 values below are placeholders updated by the release pipeline
# (or manually) for each tagged version — replace VERSION and the SHA256 values
# after a release is published. Until then `brew install` will fail the checksum
# check by design.
class Traceguard < Formula
  desc "Local black box recorder, safety layer, and patch review for AI coding agents"
  homepage "https://github.com/TaxCollector23/TraceGuard"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v0.1.0/trg-macos-arm64"
      sha256 "REPLACE_WITH_SHA256_macos_arm64"
    end
    on_intel do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v0.1.0/trg-macos-x64"
      sha256 "REPLACE_WITH_SHA256_macos_x64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v0.1.0/trg-linux-arm64"
      sha256 "REPLACE_WITH_SHA256_linux_arm64"
    end
    on_intel do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v0.1.0/trg-linux-x64"
      sha256 "REPLACE_WITH_SHA256_linux_x64"
    end
  end

  def install
    # The downloaded artifact is the bare binary; name it `trg` on install.
    binary = Dir["*"].first
    bin.install binary => "trg"
    # Provide the long alias `traceguard` pointing at the same binary.
    bin.install_symlink "trg" => "traceguard"
  end

  test do
    assert_match "trg", shell_output("#{bin}/trg --version")
  end
end
