class Lview < Formula
  desc "TUI for viewing remote logs over SSH"
  homepage "https://github.com/janvete/lview"
  url "https://github.com/janvete/lview/archive/refs/tags/v0.1.4.tar.gz"
  sha256 "382ff011dceb8a29cd40447ed0497db0475814f6d8d0ac066d64af0ae9b4fd2a"
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
