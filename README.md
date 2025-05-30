
The Rust `nodyn::`[`wrap!`] macro creates a wrapper enum for a set of
types and can generate method and trait delegation.

TODO: nice example demonstrating all features

# Values of different Types in Rust

When we want to have values of different types in Rust there are
two possible solutions: Trait Objects or Enum Wrappers. The second
option is a "good solution when our interchangeable items are a
fixed set of types that we know when our code is compiled"[^book].
// /html/book/ch18-02-trait-objects.html#using-trait-objects-that-allow-for-values-of-different-types

## Example

[Listing 8-9][Listing_8-9] from the book[^book]:
// /html/book/ch08-01-vectors.html#using-an-enum-to-store-multiple-types

```rust
    enum SpreadsheetCell {
        Int(i32),
        Float(f64),
        Text(String),
    }

    let row = vec![
        SpreadsheetCell::Int(3),
        SpreadsheetCell::Text(String::from("blue")),
        SpreadsheetCell::Float(10.12),
    ];
```

With nodyn, which implements `From` for each wrapped type:

```rust
    nodyn::wrap! {
        enum SpreadsheetCell { i32, f64, String }
    }

    let row: Vec<SpreadsheetCell> = vec![
        3.into(),
        String::from("blue").into(),
        10.12.into(),
    ];
```

The advantage of `enum` wrappers over trait objects is that there
is no type erasure and its faster.

# Downside of Enum Wrappers

However, using an `enum` wrapper requires extra code to delegate
function calls. Adding types or functions requires a lot of changes
to the enum wrapper, bigger changes in comparison to trait objects.
The [`wrap!`] macro generates the delegations and you get easy
wrapping and unwrapping with automatic implementations of
the `From` and `TryFrom` traits.

## Example

Here is [Listing 10-13][Listing_10-13] from the book[^book]:

```rust
   pub trait Summary {
       fn summarize(&self) -> String;
   }
   
   pub struct NewsArticle {
       pub headline: String,
       pub location: String,
       pub author: String,
       pub content: String,
   }
   
   impl Summary for NewsArticle {
       fn summarize(&self) -> String {
           format!("{}, by {} ({})", self.headline, self.author, self.location)
       }
   }
   
   pub struct SocialPost {
       pub username: String,
       pub content: String,
       pub reply: bool,
       pub repost: bool,
   }
   
   impl Summary for SocialPost {
       fn summarize(&self) -> String {
           format!("{}: {}", self.username, self.content)
       }
   }
```

We can create an enum Wrapper `Article` that implements `Summery`
by delegating to `NewsArticle` or `SocialPost`:

```rust
nodyn::wrap! {
    enum Article {NewsArticle, SocialPost}

    impl Summary {
        fn summarize(&self) -> String;
    }
}
```

See the documentation of the [`wrap!`] macro for details.

# Alternative crates

- **[enum_dispatch]**
    - can only generate delegation for traits in scope
      (but in a very convenient way).
- **[sum_type]**
    - very limited to the type of types being wrapped (e.g. no lifetimes)
    - no delegation

[enum_dispatch]: https://crates.io/crates/enum_dispatch
[sum_type]: https://crates.io/crates/sum_type

[^book]: "The Rust Programming Language" by Steve Klabnik, Carol Nichols, and Chris Krycho, with contributions from the Rust Community

[Listing_8-9]: http://localhost:3000/share/rust/html/book/ch08-01-vectors.html#listing-8-9
[Listing_10-13]: http://localhost:3000/share/rust/html/book/ch10-02-traits.html#listing-10-13
