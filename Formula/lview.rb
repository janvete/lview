class Lview < Formula
  desc "TUI for viewing remote logs over SSH"
  homepage "https://github.com/janvete/lview"
  url "https://github.com/janvete/lview/archive/refs/tags/v0.1.1.tar.gz"
  sha256 "ec83436ce29602df7cfbccb554a0e4ed41e3a90776ea9117de91e0e8a8638248"
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
