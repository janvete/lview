class Lview < Formula
  desc "TUI for viewing remote logs over SSH"
  homepage "https://github.com/janvete/lview"
  url "https://github.com/janvete/lview/archive/refs/tags/v0.1.3.tar.gz"
  sha256 "364787dc5ff1c81bdbc16d4cf3ebfa716f9cfa130357c02cfda2d20a12c3efd0"
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
