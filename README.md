# Networking Wrappers

This is a utility crate that offers async wrappers for using networking utilities, for example to create
network namespaces, bridge devices, marking networking interfaces as up, managing interface addresses,
moving network interfaces into bridges, creating virtual ethernet pairs and wireguard interfaces, and
managing iptables rules.

Use this crate if you want to interact with low-level networking utilities in an async codebase.

Resources:
- Documentation: [nightly][rustdoc], [latest release][docs]
- Crates.io: [fractal-networking-wrappers][cratesio]

## Usage

To use this crate, simply add this line to your dependencies section in your crate configuration:

```
fractal-networking-wrappers = "0.1"
```

## Optional features

There are no optional features.

## License

[AGPL 3.0](LICENSE.md), commercial licensing available upon request.

[rustdoc]: https://fractalnetworks.gitlab.io/libraries/networking-wrappers/doc/fractal_networking_wrappers
[docs]: https://docs.rs/fractal-networking-wrappers
[cratesio]: https://crates.io/crates/fractal-networking-wrappers
