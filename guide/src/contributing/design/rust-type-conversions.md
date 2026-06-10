# Rust Type conversions

Previously we've been seeing mostly abridged versions of type conversions when
values enter Rust. Here we'll go into some more depth about how this is
implemented. There are two categories of traits for converting values, traits
for converting values from Rust to JS and traits for the other way around.

## From Rust to JS

First up let's take a look at going from Rust to JS:

```rust
pub trait IntoWasmAbi: WasmDescribe {
    type Abi: WasmAbi;
    fn into_abi(self) -> Self::Abi;
}
```

And that's it! This is actually the only trait needed currently for translating
a Rust value to a JS one. There's a few points here:

* We'll get to `WasmDescribe` later in this section.

* The associated type `Abi` is the type of the raw data that we actually want to pass to JS.
  The bound `WasmAbi` is implemented for primitive types like `u32` and `f64`,
  which can be represented directly as WebAssembly values, as well of a couple
  of other types like `WasmSlice`:

  ```rust
  pub struct WasmSlice {
      pub ptr: u32,
      pub len: u32,
  }
  ```

  This struct, which is how things like strings are represented in FFI, isn't
  a WebAssembly primitive type, and so it can't be mapped directly to a
  WebAssembly parameter / return value. This is why `WasmAbi` lets types specify
  how they can be split up into multiple WebAssembly parameters:

  ```rust
  impl WasmAbi for WasmSlice {
      fn split(self) -> (u32, u32, (), ()) {
          (self.ptr, self.len, (), ())
      }

      // some other details to specify return type of `split`, go in the other direction
  }
  ```

  This means that a `WasmSlice` gets split up into two `u32` parameters.
  The extra unit types on the end are there because Rust doesn't let us make
  `WasmAbi` generic over variable-length tuples, so we just take tuples of 4
  elements. The unit types still end up getting passed to/from JS, but the C ABI
  just completely ignores them and doesn't generate any arguments.

  Since we can't return multiple values, when returning a `WasmSlice` we instead
  put the two `u32`s into a `#[repr(C)]` struct and return that.

* And finally we have the `into_abi` function, returning the `Abi` associated
  type which will be actually passed to JS.

This trait is implemented for all types that can be converted to JS and is
unconditionally used during codegen. For example you'll often see `IntoWasmAbi
for Foo` but also `IntoWasmAbi for &'a Foo`.

The `IntoWasmAbi` trait is used in two locations. First it's used to convert
return values of Rust exported functions to JS. Second it's used to convert the
Rust arguments of JS functions imported to Rust.

## From JS to Rust

Going from JS to Rust, every value an exported function receives is
converted through a single trait:

```rust
pub trait ArgAbi<S: Scope> {
    type Abi: WasmAbi;
    type Guard;
    type Projected<'a> where Self: 'a;
    unsafe fn arg_from_abi(abi: Self::Abi) -> Self::Guard;
    fn project<'a>(guard: &'a mut Self::Guard) -> Self::Projected<'a> where Self: 'a;
    fn describe_arg();
}

pub trait OptionArgAbi<S: Scope>: Sized {
    type OptionAbi: WasmAbi;
    unsafe fn option_arg_from_abi(abi: Self::OptionAbi) -> Option<Self>;
}
```

`ArgAbi<S>` is dispatched on the *written* argument type — which is what
makes type aliases work, since trait resolution sees through aliases where
the macro's view of the syntax cannot. Owned values, references, and
`Option`s are all plain impls of this one trait:

* **Owned types** (primitives, `String`, `Vec<T>`, `JsValue`, exported
  structs and enums by value, …) use the decoded value itself as the
  `Guard` and hand it over in `project`.
* **References** like `&str`, `&[u8]`, `&mut [u8]`, `&JsValue`, and
  `&ExportedStruct` decode into an owning `Guard` (e.g. `Box<str>`, a
  reference-counted class anchor) and project a borrow of it; the
  generated code drops guards once the call — or, for `async` exports,
  the returned future — completes. The `Scope` parameter distinguishes
  those two cases: `CallScoped` borrows live for one synchronous call,
  while `Anchored` guards own their data so projections can live across
  `.await` points.
* **`Option<T>`** is a single generic impl gated on `T: OptionArgAbi<S>`:
  the orphan rules prevent downstream crates from implementing anything
  for `Option<TheirType>`, so the `None` encoding is a capability of the
  *element* — `#[wasm_bindgen]` emits the `OptionArgAbi` impl alongside
  the type, and types without an `Option` encoding simply don't implement
  it.

Imported functions' return values and the `Ok` side of
`#[wasm_bindgen(catch)]` results use the by-value case of the same trait
(`Guard = Option<Self>`), the latter via `CatchFromWasmAbi`, which checks
the thread-local exception state before converting.
