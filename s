cargo build --target=aarch64-unknown-linux-musl --release -Z build-std=panic_abort,std  -Z build-std-features=panic_immediate_abort
/mnt/bkel/android/androidsdk/ndk-bundle/toolchains/aarch64-linux-android-4.9/prebuilt/linux-x86_64/bin/aarch64-linux-android-strip target/aarch64-unknown-linux-musl/release/andrgesture -o andrgesture
scp andrgesture pix3:./
