# typed: false
# frozen_string_literal: true

# Syncara Homebrew formula
#
# This file is a TEMPLATE. Each release generates the actual formula
# as a release artifact (syncara.rb). To use the bottled version:
#
#   brew install https://github.com/anomalyco/syncara/releases/download/v0.1.0/syncara.rb
#
# For a permanent tap, see https://github.com/anomalyco/homebrew-tap
#   brew tap anomalyco/tap
#   brew install syncara

class Syncara < Formula
  desc "Smart Traffic Brain — fast, deterministic reverse proxy and load balancer"
  homepage "https://github.com/anomalyco/syncara"
  license "MIT"

  unless build.head?
    version "VERSION_PLACEHOLDER"
  end

  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/anomalyco/syncara/releases/download/vVERSION_PLACEHOLDER/syncara-VERSION_PLACEHOLDER-aarch64-apple-darwin.tar.gz"
      sha256 "MACOS_ARM64_SHA256_PLACEHOLDER"
    else
      url "https://github.com/anomalyco/syncara/releases/download/vVERSION_PLACEHOLDER/syncara-VERSION_PLACEHOLDER-x86_64-apple-darwin.tar.gz"
      sha256 "MACOS_X86_64_SHA256_PLACEHOLDER"
    end
  end

  if OS.linux?
    if Hardware::CPU.arch == :arm64
      url "https://github.com/anomalyco/syncara/releases/download/vVERSION_PLACEHOLDER/syncara-VERSION_PLACEHOLDER-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "LINUX_ARM64_SHA256_PLACEHOLDER"
    else
      url "https://github.com/anomalyco/syncara/releases/download/vVERSION_PLACEHOLDER/syncara-VERSION_PLACEHOLDER-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "LINUX_X86_64_SHA256_PLACEHOLDER"
    end
  end

  def install
    bin.install "syncara"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/syncara --version")
  end
end
