with import <nixpkgs> {};
mkShell {
  buildInputs = [
    openssl
    pkgconfig
    libevdev
    libiconv
  ];
}
