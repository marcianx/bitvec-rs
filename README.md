# bitvec-rs

This is a bit vector implementation with guaranteed `[u8]` [LSB 0][1]
representation and the ability to get safe immutable and mutable views into its
internal vector for easy I/O.

[1]: https://en.wikipedia.org/wiki/Bit_numbering#LSB_0_bit_numbering

It mirrors the API of `std::vec::Vec` as much as possible. Notable differences:
- `BitVec`'s non-consuming iterator enumerates `bool`s instead of `&bool`s.

## License

Copyright 2019, Ashish Myles (maintainer) and contributors.
This software is dual-licensed under the [MIT](LICENSE-MIT) and
[Apache 2.0](LICENSE-APACHE) licenses.
