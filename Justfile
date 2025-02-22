patch:
    cargo build --release && patchelf --set-interpreter /usr/lib64/ld-linux-x86-64.so.2 target/release/recopilatori
