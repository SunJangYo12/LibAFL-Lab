# Tools
  build and run in termux android aarch64
  cargo-make version 0.34.0
  rustc version rustc 1.82.0 (f6e511eec 2024-10-15) termux repository
  clang version 19.1.4

# build & run
	$ cargo make fuzzer
	$ cargo make run #edit Makefile.toml comment build lib

# run all core
  term 1
	$ ./fuzzer_libpng 
  term 2
	$ ./fuzzer_libpng 2>/dev/null
