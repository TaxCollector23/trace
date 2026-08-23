# Trace Homebrew formula.
#
# Installs the `trace` binary from GitHub Releases.
# Homebrew only evaluates the block for the current platform, so each platform's
# sha256 is independent.
class Trace < Formula
  desc "The trust layer for autonomous software engineering"
  homepage "https://github.com/TaxCollector23/trace"
  version "1.3.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.0/trace-macos-arm64"
      sha256 "52dc02beb4ae74bd5c72639d50b4405a63fc264a0699274bbacf85187ad387cd"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.0/trace-macos-x64"
      sha256 "a9d88e602e61b39a11d779b90c05a7d8473156371f0abdfdaabeac2583790e49"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.0/trace-linux-arm64"
      sha256 "5bd423ad38d3c43e5e75fa3eb3e0028dda0f7a31756c281528311c2a4c46cbde"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.0/trace-linux-x64"
      sha256 "cbaa8a46c03ce7e7851e157f3719fdace62cd5ec34345220d637e97de30538bc"
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
