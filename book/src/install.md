# Installing Graphix

To install the Graphix shell from source you need to install a rust build
environment. See [here](https://www.rust-lang.org/tools/install) for
instructions on how to do that for your platform. Once you have that set up, you
can just run

`cargo install graphix-shell`

That should build the `graphix` command and install it in your ~/.cargo/bin
directory. On linux you may need to install kerberos headers, as well as clang
libs for gssapi to build properly (on linux). On debian/ubuntu install
`libclang-dev`, and `libkrb5-dev`. On other distributions the names will be
similar.
