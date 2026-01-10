# Homebrew formula for AllBeads
# To use this formula, create a tap repository and copy this file there.
#
# Example tap setup:
#   1. Create a repository named "homebrew-allbeads" on GitHub
#   2. Add this file as Formula/allbeads.rb
#   3. Users install with: brew tap thrashr888/allbeads && brew install allbeads
#
# Update the VERSION and SHA256 values when releasing a new version.

class Allbeads < Formula
  desc "Distributed protocol for agentic orchestration and communication"
  homepage "https://github.com/thrashr888/AllBeads"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-macos-aarch64"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"
    end
    on_intel do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-macos-x86_64"
      sha256 "PLACEHOLDER_SHA256_MACOS_X86_64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-linux-aarch64"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-linux-x86_64"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  def install
    bin.install Dir["allbeads*"].first => "ab"
  end

  def caveats
    <<~EOS
      AllBeads has been installed as 'ab'.

      Quick start:
        ab init                  # Initialize configuration
        ab context add           # Add current repo as context
        ab stats                 # View aggregated statistics
        ab tui                   # Launch interactive dashboard

      For AI agents:
        ab quickstart            # Quick start guide
        ab prime                 # Prime agent context
        ab info                  # Project overview

      Documentation: https://github.com/thrashr888/AllBeads
    EOS
  end

  test do
    system "#{bin}/ab", "--version"
  end
end
