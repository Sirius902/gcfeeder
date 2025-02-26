set windows-shell := ["powershell.exe", "-c"]

default: fmt

fmt:
  @cargo fmt
  @taplo fmt
