# TraceGuard Homebrew formula.
#
# Installs the `trg` binary from GitHub Releases and adds a `traceguard` alias.
# Homebrew only evaluates the block for the current platform, so each platform's
# sha256 is independent.
#
# NOTE: the macOS Intel (x86_64) asset can lag the others when GitHub's macОS-13
# runners are queued. Apple Silicon and Linux install immediately. Intel-mac
# users can use the curl installer until the x64 sha is filled:
#   curl -fsSL https://raw.githubusercontent.com/TaxCollector23/TraceGuard/main/scripts/install.sh | sh
class Traceguard < Formula
  desc "Local black box recorder, safety layer, and TraceCompress for AI coding agents"
  homepage "https://github.com/TaxCollector23/TraceGuard"
  version "1.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v1.1.0/trg-macos-arm64"
      sha256 "75c091080d2fcb6f32530915460a9850f5238de2f05d662c4fa5d3b292b63153"
    end
    on_intel do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v1.1.0/trg-macos-x64"
      # Pending: macOS Intel asset publishes after the macОS-13 runner dequeues.
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v1.1.0/trg-linux-arm64"
      sha256 "4deeb2152877f918c107220ff37608e7a9ecf1f78941fb8cb2b02ba5a086f548"
    end
    on_intel do
      url "https://github.com/TaxCollector23/TraceGuard/releases/download/v1.1.0/trg-linux-x64"
      sha256 "23dd27e6314ddfcfd16727156f24290efb1bc3eaa6864a78f29a5d8d2ad17b4b"
    end
  end

  def install
    # The downloaded artifact is the bare binary; install it as `trg`.
    binary = Dir["*"].first
    bin.install binary => "trg"
    # Provide the guaranteed-safe long alias `traceguard`.
    bin.install_symlink "trg" => "traceguard"
  end

  def caveats
    <<~EOS
      TraceGuard installed two commands:
        trg         (short)
        traceguard  (long alias — use this if another `trg` exists on your PATH)

      Start the local dashboard with:
        trg dashboard
    EOS
  end

  test do
    assert_match "TraceGuard 1.1", shell_output("#{bin}/trg --version")
    assert_match "TraceGuard 1.1", shell_output("#{bin}/traceguard --version")
  end
end
