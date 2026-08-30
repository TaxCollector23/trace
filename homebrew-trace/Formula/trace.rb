# Trace Homebrew formula.
#
# Installs the `trace` binary from GitHub Releases.
# Homebrew only evaluates the block for the current platform, so each platform's
# sha256 is independent.
class Trace < Formula
  desc "The trust layer for autonomous software engineering"
  homepage "https://github.com/TaxCollector23/trace"
  version "1.3.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.1/trace-macos-arm64"
      sha256 "40fa18c5425fe32b64da17d3f13e45fbc57a0b1983671e217c0eee9723506213"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.1/trace-macos-x64"
      sha256 "dab01c78c2957902117244582b9abc6ce9237178bb28d68ea74abfbc42381cb6"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.1/trace-linux-arm64"
      sha256 "c9c6ab460037411bcec9594be7719b22c4051ee81f34f1614c931ba74e2f8c3b"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.3.1/trace-linux-x64"
      sha256 "1e1a5c79f7f9fdf09296e797abf3f8ec4ee3ac89806e78aee1d8c2eba807d4ae"
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
