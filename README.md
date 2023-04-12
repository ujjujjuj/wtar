# wtar: worse tar

A bare-bones [tar](<https://en.wikipedia.org/wiki/Tar_(computing)>) clone, written in Rust.

## Usage

### Creating a compressed archive

```sh
wtar -c infolder/
```

This will create an archive called `infolder.wtar.gz` in the same directory

### Extracting an archive

```sh
wtar -e archive.wtar.gz
```

## Building

This is a cargo project. To build it in release mode, simply run

```sh
cargo build --release
```
