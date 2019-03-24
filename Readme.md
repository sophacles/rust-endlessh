# rust-endlessh

This is a rust version of the ssh tarpit created by Chris Wellons (see here:
https://nullprogram.com/blog/2019/03/22/ ). This started as a direct clone, and
I added a bit of output to keep track of what's been caught. It's built using
tokio for async networking, and the rand crate to generate noise.

It works and is super lightweight. To use just clone the repository
and do:

```
$ cargo build --release
$ sudo ./target/release/sshtarpit
```

