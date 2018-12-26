# unbox

A work in progress command line utility to unpack various types of archives quickly.

```
unbox /path/to/my/archive.zip
```

<img src="https://raw.githubusercontent.com/mitsuhiko/unbox/master/unbox.gif">

## Installation

```
cargo install unbox
```

## Supported Formats

- unix ar archives
- zip archives
- uncompressed tarballs
- gzip-compressed tarballs
- xz-compressed tarballs
- bzip2-compressed tarballs
- gzip-compressed files
- xz-compressed files
- bzip2-compressed files

## FAQ

**Why do this?**

> No specific reason.  I used to have a Python tool called [unp](https://github.com/mitsuhiko/unp)
> which just shells out to system tools to unpack and I felt like I want to see if I can use the
> rust ecosystem to build one that comes with the unpacking code.

**How fast is it?**

> It's not particularly fast.  In fact it's about 50% slower than the system tools but for most
> archives I unpack that does not cause me any grief.
