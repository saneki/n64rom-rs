# `n64rom`

A library and toolkit for working with N64 rom files.

## Installation

Requires Rust nightly. You can use `rustup` to install the toolchain:

```bash
rustup toolchain install nightly
```

Install `n64rom`:

```bash
cargo +nightly install n64rom
```

## Tools

`n64romtool` is a provided utility for inspecting N64 rom files.

Currently it can:
- Show info about the rom's header and IPL3.
- Convert the rom to a different byte order.
- Verify the CRC values in the rom header.
- Correct the CRC values in the rom header.

To install `n64romtool`, run:

```bash
cargo +nightly install n64rom --features=n64romtool
```

Some usage examples:

```bash
# Display info about rom file "MyRom.z64"
n64romtool show MyRom.z64

# Convert rom file "MyRom.z64" to big-endian byte order (easiest to read)
# You can convert to: [big, little, mixed]
n64romtool convert big MyRom.z64 MyRomBig.z64

# Verify the CRC values in rom file "MyRom.z64"
n64romtool check MyRom.z64

# Correct the CRC values in rom file "MyRom.z64"
n64romtool correct MyRom.z64
```
