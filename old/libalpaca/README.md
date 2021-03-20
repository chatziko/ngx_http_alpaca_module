# libalpaca [![Travis branch](https://img.shields.io/travis/camelids/libalpaca/master.svg)](https://travis-ci.org/camelids/libalpaca) [![Codecov branch](https://img.shields.io/codecov/c/github/camelids/libalpaca/master.svg)](https://codecov.io/gh/camelids/libalpaca)

[Crate Documentation](https://camelids.github.io/libalpaca/master/alpaca/) |
[design/specs.txt](https://github.com/camelids/libalpaca/blob/master/design/specs.txt) |
[Paper](https://www.degruyter.com/view/j/popets.2017.2017.issue-2/popets-2017-0023/popets-2017-0023.xml)

:construction: WARNING: The code is currently under construction and not safe to use. :construction:

This library implements the ALPaCA website fingerprinting defense, and is intended for use in the creation of web server modules.



<p align="center">
  <img src="/design/sample-site/alpacas-in-a-field.jpg">
</p>

## Compilation

In order to compile the library, Rust should be installed in your computer. Download the repository, move inside the folder
and run `cargo build --release`. The `libalpaca.so` file is created in the `target/release/` folder.
