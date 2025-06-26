cargo build --release
clang++ -DNO_MAIN -g -O2 -fsanitize-coverage=trace-pc-guard -fsanitize=address -Wl,--whole-archive target/release/libappsec_guide.a -Wl,--no-whole-archive main.cc harness.cc -o fuzz
