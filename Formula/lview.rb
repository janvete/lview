class Lview < Formula
  desc "TUI for viewing remote logs over SSH"
  homepage "https://github.com/janvete/lview"
  url "https://github.com/janvete/lview/archive/refs/tags/v0.1.7.tar.gz"
  sha256 "7bb92852698486571a5c31158f64b398d123d3c7ebb1982733c43e0e1c84209a"
  license "MIT"
  head "https://github.com/janvete/lview.git", branch: "master"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "TUI for viewing remote logs over SSH", shell_output("#{bin}/lview --help")
  end
end
