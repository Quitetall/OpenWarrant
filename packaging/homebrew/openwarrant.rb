# SPDX-License-Identifier: Apache-2.0
#
# TEMPLATE for the formula in the tap repository Quitetall/homebrew-openwarrant
# (Formula/openwarrant.rb). Do not copy this file there by hand: render it for a
# published release, which fills every @-delimited placeholder from that release's own
# .sha256 assets and refuses a release that lacks one:
#
#   python3 packaging/homebrew/render.py --tag v1.0.0-alpha.3 > Formula/openwarrant.rb
#
# It installs the release workflow's prebuilt archives (package.py's naming),
# for the two targets release.yml builds: macOS on Apple silicon and Linux
# x86_64. Anything else has no bottle or archive; build from source instead
# (docs/INSTALL.md).
class Openwarrant < Formula
  desc "Work Authorization Records: the `war` command-line interface"
  homepage "https://github.com/@REPO@"
  version "@VERSION@"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/@REPO@/releases/download/@TAG@/openwarrant-@TAG@-darwin-arm64.tar.gz"
      sha256 "@SHA256_DARWIN_ARM64@"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/@REPO@/releases/download/@TAG@/openwarrant-@TAG@-linux-x86_64.tar.gz"
      sha256 "@SHA256_LINUX_X86_64@"
    end
  end

  def install
    # The archive is flat: bin/war beside LICENSE, THIRD_PARTY.json and
    # licenses/ (the dependency notices that must travel with the binary).
    bin.install "bin/war"
    prefix.install "LICENSE", "THIRD_PARTY.json", "licenses"
  end

  test do
    # A release build prints a bare `war <version>` (docs/INSTALL.md).
    assert_equal "war #{version}", shell_output("#{bin}/war --version").strip
  end
end
