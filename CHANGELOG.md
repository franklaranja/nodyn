# Changelog

All notable changes to this project will be documented in this file.

## v0.2.0

### Feature selection changed, old way depriciated.

Parts of code generated could be configured using cargo feature flags.
Now this can be selected for each enum using impl followed by one or
more feature names, you can have multiple of these impl's. `impl introspection;`
or `impl is_as TryInto;`. The name `try_into` has changed to `TryInto` when
using impl for selection. The from feature does no longer exist.

The using cargo for selecting is depriciated. For now they are still
used if no features have been enabled using impl.

### `Vec` wrappers

Nodyn now can generate a `Vec` wrapper for your enum. It has adapted methods
for example it is possible to push values that the enum wraps directly to
the vec without having to call `into()`.

```rust
nodyn::nodyn! {
  #[derive(Debug, PartialEq, Clone)]
  pub enum Foo<'a> {
    &'a str,
    u32,
  }

  impl vec;
}

fn main() {
    let regular_vec: Vec<Foo> = vec!["a".into(), 3.into()];
    let mut bar: FooVec = regular.into();

    bar.push(42u32);
    bar.push("hello");

    for t in bar.iter_str_ref() {
        // iterate over &str values
    }

    let baz = foo_vec!["b", "c", 33];
}
```

## [v0.1.0](https://github.com/franklaranja/nodyn/releases/tag/v0.1.0) - 2025-5-2

Initial release
