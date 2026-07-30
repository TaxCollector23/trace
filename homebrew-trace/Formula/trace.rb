# Trace Homebrew formula.
#
# Installs the `trace` binary from GitHub Releases.
# Homebrew only evaluates the block for the current platform, so each platform's
# sha256 is independent.
class Trace < Formula
  desc "The trust layer for autonomous software engineering"
  homepage "https://github.com/TaxCollector23/trace"
  version "1.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.2.0/trace-macos-arm64"
      sha256 "986a6fa5f8ce69431b351cf11c9dda783c7727422fc238a37122238cb20353c6"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.2.0/trace-macos-x64"
      sha256 "8f7689ba7a2e9dfc305d5731d686a27e32d5f3876d46dfdde4b4df2044fe9a89"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.2.0/trace-linux-arm64"
      sha256 "083d0a6dd6e7dbbc55c7622ed4c74dd456e9cd6f8def5c00805d1d0ab166380e"
    end
    on_intel do
      url "https://github.com/TaxCollector23/trace/releases/download/v1.2.0/trace-linux-x64"
      sha256 "ee96c975a7fb886a5901eeecdce4962e656aad412ccfb3c3c000c8d8f41c4e7b"
    end
  end

  def install
    # The downloaded artifact is the bare binary; install it as `trace`.
    binary = Dir["*"].first
    bin.install binary => "trace"
  end

  def caveats
    <<~EOS
      Start the local dashboard with:
        trace dashboard
    EOS
  end

  test do
    assert_match "Trace 1.2", shell_output("#{bin}/trace --version")
  end
end
