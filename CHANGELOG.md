# Changelog

All notable changes to this project will be documented in this file.

## v0.2.0

### Feature selection changed, old way depriciated.

Parts of code generated could be configured using cargo feature flags.
Now features can selected for each enum using impl followed by one or
more feature names, you can habe multiple of these impl's. `impl From;`
or `impl is_as TryInto From;` as you can see the names `from` and
`try_into` have changed to camel case for impl use.

The using cargo for selecting is depriciated. For now they are still
used if no features have been enabled using impl.

## [v0.1.0](https://github.com/franklaranja/nodyn/releases/tag/v0.1.0) - 2025-5-2

Initial release
