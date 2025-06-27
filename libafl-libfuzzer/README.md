# Tools
  build and run in termux android aarch64
  cargo-make version 0.34.0
  rustc version rustc 1.82.0 (f6e511eec 2024-10-15) termux repository
  clang version 19.1.4
BUG: di termux akan force close jika menemukan bug, memori terlalu
     kecil untuk menggunakan asan.

# build & run
	$ cargo make fuzzer
	$ cargo make run #edit Makefile.toml comment build lib
	$ cargo make run
BUG: in make install, fix this libtiff/doc/Makefile comment install-data-local

# run all core
  term 1
	$ ./fuzzer_libexif
  term 2
	$ ASAN_OPTIONS=abort_on_error=1 ./fuzzer_libexif

