# Gravure - an image resizing microservice #

## Description ##
Our previous solution of storing images of different sizes was unscalable and split over multiple language and server
configurations. To resolve this situation we wrote a microservice that is able to receive an image, apply specified
conversions (like resize, crop, watermark, add/remove meta, etc) and upload it back to configured location.

One of the main goals was to make it convenient to change image conversion profiles.

Such service was written in Rust at one of the internal hackatons at Avito.

## Setup

- You will need rust building environment. It'is usually set up from your package manager or via [Rustup](https://www.rustup.rs/)
- After you've got Rust, just run
    `cargo build --release`
    and your binary is ready at `target/release` directory.

## Notes
If you have trouble with openssl on MacOS, do the following (you'll need [brew](http://brew.sh/)):
```bash
brew install openssl
export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib
cargo clean
cargo build
```
