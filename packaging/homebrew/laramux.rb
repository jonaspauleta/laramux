# typed: false
# frozen_string_literal: true

class Laramux < Formula
  desc "TUI for managing Laravel development processes"
  homepage "https://github.com/jonaspauleta/laramux"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/jonaspauleta/laramux/releases/download/v#{version}/laramux-macos-aarch64"
      sha256 "PLACEHOLDER_MACOS_ARM64_SHA256"

      def install
        bin.install "laramux-macos-aarch64" => "laramux"
      end
    end

    on_intel do
      url "https://github.com/jonaspauleta/laramux/releases/download/v#{version}/laramux-macos-x86_64"
      sha256 "PLACEHOLDER_MACOS_X86_64_SHA256"

      def install
        bin.install "laramux-macos-x86_64" => "laramux"
      end
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/jonaspauleta/laramux/releases/download/v#{version}/laramux-linux-aarch64"
      sha256 "PLACEHOLDER_LINUX_ARM64_SHA256"

      def install
        bin.install "laramux-linux-aarch64" => "laramux"
      end
    end

    on_intel do
      url "https://github.com/jonaspauleta/laramux/releases/download/v#{version}/laramux-linux-x86_64"
      sha256 "PLACEHOLDER_LINUX_X86_64_SHA256"

      def install
        bin.install "laramux-linux-x86_64" => "laramux"
      end
    end
  end

  test do
    assert_match "laramux", shell_output("#{bin}/laramux --help 2>&1", 1)
  end
end
