class Lview < Formula
  desc "TUI for viewing remote logs over SSH"
  homepage "https://github.com/janvete/lview"
  url "https://github.com/janvete/lview/archive/refs/tags/v0.1.6.tar.gz"
  sha256 "577b10aee2f5da89f724f03fea37f31135eff06f32cce0da0fd3756466902b7d"
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
