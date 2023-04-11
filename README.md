# wtar: worse tar

A bare-bones [tar](<https://en.wikipedia.org/wiki/Tar_(computing)>) clone, written in Rust.

## Usage

Creating an archive

```sh
wtar -c archive.wtar infolder/
```

Extracting an archive

```sh
wtar -e archive.wtar
```

## Building

This is a cargo project. To build it, simply run

```sh
cargo build --release
```
