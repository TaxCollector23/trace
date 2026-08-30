# Trace Homebrew formula.
#
# Installs the `trace` binary from GitHub Releases.
# Homebrew only evaluates the block for the current platform, so each platform's
# sha256 is independent.
class Trace < Formula
  desc "The trust layer for autonomous software engineering"
  homepage "https://github.com/TaxCollector23/trace"
  version "1.3.2"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.2/trace-macos-arm64"
      sha256 "e76cec725cb3494ffebc2f81a98f8eb8f8903b226ea7017952b99fc9a22939a8"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.2/trace-macos-x64"
      sha256 "a019d91f2b9e47ff9f06a9b957e122b001dba69e684ad7fe5251dfb224ab6a87"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.2/trace-linux-arm64"
      sha256 "e994bc03a0ba50519602cdf7e64338855125d947c4222ad05a99f603c0bb2227"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.2/trace-linux-x64"
      sha256 "3e2b4bf106ca50f075d9a2ce789dcaf320e8490466aed365a6fc2c02afcf8ea8"
    end
  end

  def install
    # The downloaded artifact is the bare binary; install it as `trace`.
    binary = Dir["*"].first
    bin.install binary => "trc"
  end

  def caveats
    <<~EOS
      Start the local dashboard with:
        trc dashboard
    EOS
  end

  test do
    assert_match "Trace 1.3", shell_output("#{bin}/trc --version")
  end
end
