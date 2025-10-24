# Installing Graphix

To install the Graphix shell from source you need to install a rust build
environment. See [here](https://www.rust-lang.org/tools/install) for
instructions on how to do that for your platform. Once you have that set up, you
can just run

`cargo install graphix-shell`

That should build the `graphix` command and install it in your
~/.cargo/bin directory. Windows and Mac OS should work out of the box
as long as you have the prerequisites for rust installed.

## Linux Prerequisites

### Debian/Ubuntu

You need to install

- clang
- libkrb5-dev

### On Fedora

You need to install

- clang-devel
- krb5-devel

## Netidx

Graphix uses netidx to import and export data streams. So it is
recommended that you set up at least a machine local installation of
netidx when installing Graphix. Otherwise separate Graphix processes
won't be able to communicate with each other and it will be difficult
to get any data into Graphix.

See [here](https://netidx.github.io/netidx-book) for details

If you don't want to set up netidx Graphix will still work, it just
won't be able to use the net module to send anything outside the
current process.
