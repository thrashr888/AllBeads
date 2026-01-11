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
  version "0.2.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-macos-aarch64"
      sha256 "0416bb566e0705f71b06cabe1fc7c027525617d6390792e37d44ce5bf347f2f6"
    end
    on_intel do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-macos-x86_64"
      sha256 "25c0d72f4fee09dc59bfd4b1c8aaa39fe629877251631c40956de43e6ad1248a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-linux-aarch64"
      sha256 "49ac022e18679b806f1dce6fdc5594c14151fcee31ba88831f8cbf25cccc692c"
    end
    on_intel do
      url "https://github.com/thrashr888/AllBeads/releases/download/v#{version}/allbeads-linux-x86_64"
      sha256 "258ff2c11adcfd78910f63ea3ee356d4228e5f60f654324d9dae3c59f530d3b6"
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
