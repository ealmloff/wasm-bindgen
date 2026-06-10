use core::panic::AssertUnwindSafe;

use crate::sys::JsOption;
use crate::{describe::*, JsCast};
use crate::{ErasableGeneric, JsValue};

/// A trait for anything that can be converted into a type that can cross the
/// Wasm ABI directly, eg `u32` or `f64`.
///
/// This is the opposite operation of `ArgAbi`.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub trait IntoWasmAbi: WasmDescribe {
    /// The Wasm ABI type that this converts into when crossing the ABI
    /// boundary.
    type Abi: WasmAbi;

    /// Convert `self` into `Self::Abi` so that it can be sent across the wasm
    /// ABI boundary.
    fn into_abi(self) -> Self::Abi;
}

/// Indicates that this type can be passed to JS as `Option<Self>`.
///
/// This trait is used when implementing `IntoWasmAbi for Option<T>`.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub trait OptionIntoWasmAbi: IntoWasmAbi {
    /// Returns an ABI instance indicating "none", which JS will interpret as
    /// the `None` branch of this option.
    ///
    /// It should be guaranteed that the `IntoWasmAbi` can never produce the ABI
    /// value returned here.
    fn none() -> Self::Abi;
}

/// A trait for any type which maps to a Wasm primitive type when used in FFI
/// (`i32`, `i64`, `f32`, or `f64`).
///
/// This is with the exception of `()` (and other zero-sized types), which are
/// also allowed because they're ignored: no arguments actually get added.
///
/// # Safety
///
/// This is an unsafe trait to implement as there's no guarantee the type
/// actually maps to a primitive type.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub unsafe trait WasmPrimitive: Default {}

unsafe impl WasmPrimitive for u32 {}
unsafe impl WasmPrimitive for i32 {}
unsafe impl WasmPrimitive for u64 {}
unsafe impl WasmPrimitive for i64 {}
unsafe impl WasmPrimitive for f32 {}
unsafe impl WasmPrimitive for f64 {}
unsafe impl WasmPrimitive for () {}

/// A trait which represents types that can be passed across the Wasm ABI
/// boundary, by being split into multiple Wasm primitive types.
///
/// Up to 4 primitives are supported; if you don't want to use all of them, you
/// can set the rest to `()`, which will cause them to be ignored.
///
/// You need to be careful how many primitives you use, however:
/// `Result<T, JsValue>` uses up 2 primitives to store the error, and so it
/// doesn't work if `T` uses more than 2 primitives.
///
/// So, if you're adding support for a type that needs 3 or more primitives and
/// is able to be returned, you have to add another primitive here.
///
/// There's already one type that uses 3 primitives: `&mut [T]`. However, it
/// can't be returned anyway, so it doesn't matter that
/// `Result<&mut [T], JsValue>` wouldn't work.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub trait WasmAbi {
    type Prim1: WasmPrimitive;
    type Prim2: WasmPrimitive;
    type Prim3: WasmPrimitive;
    type Prim4: WasmPrimitive;

    /// Splits this type up into primitives to be sent over the ABI.
    fn split(self) -> (Self::Prim1, Self::Prim2, Self::Prim3, Self::Prim4);
    /// Reconstructs this type from primitives received over the ABI.
    fn join(prim1: Self::Prim1, prim2: Self::Prim2, prim3: Self::Prim3, prim4: Self::Prim4)
        -> Self;
}

/// A trait representing how to interpret the return value of a function for
/// the Wasm ABI.
///
/// This is very similar to the `IntoWasmAbi` trait and in fact has a blanket
/// implementation for all implementors of the `IntoWasmAbi`. The primary use
/// case of this trait is to enable functions to return `Result`, interpreting
/// an error as "rethrow this to JS"
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub trait ReturnWasmAbi: WasmDescribe {
    /// Same as `IntoWasmAbi::Abi`
    type Abi: WasmAbi;

    /// Same as `IntoWasmAbi::into_abi`, except that it may throw and never
    /// return in the case of `Err`.
    fn return_abi(self) -> Self::Abi;
}

impl<T: IntoWasmAbi> ReturnWasmAbi for T {
    type Abi = T::Abi;

    #[inline]
    fn return_abi(self) -> Self::Abi {
        self.into_abi()
    }
}

use alloc::boxed::Box;
use core::marker::Sized;

/// Trait for element types to implement IntoWasmAbi for vectors of
/// themselves.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub trait VectorIntoWasmAbi: WasmDescribeVector + Sized {
    type Abi: WasmAbi;

    fn vector_into_abi(vector: Box<[Self]>) -> Self::Abi;
}

/// Trait for element types to implement by-value decoding for vectors of
/// themselves.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub trait VectorFromWasmAbi: WasmDescribeVector + Sized {
    type Abi: WasmAbi;

    unsafe fn vector_from_abi(js: Self::Abi) -> Box<[Self]>;
}

/// A repr(C) struct containing all of the primitives of a `WasmAbi` type, in
/// order.
///
/// This is used as the return type of imported/exported functions. `WasmAbi`
/// types aren't guaranteed to be FFI-safe, so we can't return them directly:
/// instead we return this.
///
/// If all but one of the primitives is `()`, this corresponds to returning the
/// remaining primitive directly, otherwise a return pointer is used.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
#[repr(C)]
pub struct WasmRet<T: WasmAbi> {
    prim1: T::Prim1,
    prim2: T::Prim2,
    prim3: T::Prim3,
    prim4: T::Prim4,
}

impl<T: WasmAbi> From<T> for WasmRet<T> {
    fn from(value: T) -> Self {
        let (prim1, prim2, prim3, prim4) = value.split();
        Self {
            prim1,
            prim2,
            prim3,
            prim4,
        }
    }
}

// Ideally this'd just be an `Into<T>` implementation, but unfortunately that
// doesn't work because of the orphan rule.
impl<T: WasmAbi> WasmRet<T> {
    /// Joins the components of this `WasmRet` back into the type they represent.
    pub fn join(self) -> T {
        T::join(self.prim1, self.prim2, self.prim3, self.prim4)
    }
}

/// [`TryFromJsValue`] is a trait for converting a JavaScript value ([`JsValue`])
/// into a Rust type. It is used by the [`wasm_bindgen`](wasm_bindgen_macro::wasm_bindgen)
/// proc-macro to allow conversion to user types.
///
/// The semantics of this trait for various types are designed to provide a runtime
/// analog of the static semantics implemented by the IntoWasmAbi function bindgen,
/// with the exception that conversions are constrained to not cast invalid types.
///
/// For example, where the Wasm static semantics will permit `foo(x: i32)` when passed
/// from JS `foo("5")` to treat that as `foo(5)`, this trait will instead throw. Apart
/// from these reduced type conversion cases, behaviours should otherwise match the
/// static semantics.
///
/// Types implementing this trait must specify their conversion logic from
/// [`JsValue`] to the Rust type, handling any potential errors that may occur
/// during the conversion process.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub trait TryFromJsValue: Sized {
    /// Performs the conversion.
    fn try_from_js_value(value: JsValue) -> Result<Self, JsValue> {
        Self::try_from_js_value_ref(&value).ok_or(value)
    }

    /// Performs the conversion.
    fn try_from_js_value_ref(value: &JsValue) -> Option<Self>;
}

impl<WbgS: crate::convert::Scope, T> crate::convert::ArgAbi<WbgS> for AssertUnwindSafe<T>
where
    T: crate::convert::ArgAbi<WbgS, Guard = Option<T>> + WasmDescribe,
{
    type Abi = <T as crate::convert::ArgAbi<WbgS>>::Abi;
    type Guard = Option<AssertUnwindSafe<T>>;

    #[inline(always)]
    unsafe fn arg_from_abi(js: Self::Abi) -> Self::Guard {
        Some(AssertUnwindSafe(
            <T as crate::convert::ArgAbi<WbgS>>::arg_from_abi(js).unwrap(),
        ))
    }

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        <AssertUnwindSafe<T> as WasmDescribe>::describe();
    }
}

/// A trait for defining upcast relationships from a source type.
///
/// This is the inverse of [`Upcast<T>`] - instead of implementing
/// `impl Upcast<Target> for Source`, you implement `impl UpcastFrom<Source> for Target`.
///
/// # Why UpcastFrom?
///
/// This resolves Rust's orphan rule issues: you can implement `UpcastFrom<MyType>`
/// for external types when `MyType` is local to your crate, whereas implementing
/// `Upcast<ExternalType>` would be prohibited by orphan rules.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
///
/// # Relationship to Upcast
///
/// `UpcastFrom<S>` provides a blanket implementation of `Upcast<T>`:
/// ```ignore
/// impl<S, T> Upcast<T> for S where T: UpcastFrom<S> {}
/// ```
///
/// This means implementing `UpcastFrom<Source> for Target` automatically gives you
/// `Upcast<Target> for Source`, enabling `source.upcast()` to produce `Target`.
pub trait UpcastFrom<S: ?Sized> {}

/// A trait for type-safe generic upcasting.
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
///
/// # Note
///
/// `Upcast<T>` has a blanket implementation for all types where `T: UpcastFrom<Self>`.
/// New upcast relationships should typically be defined by implementing `FromUpcast`
/// rather than `Upcast` directly, to avoid orphan rule issues.
pub trait Upcast<T: ?Sized> {
    /// Perform a zero-cost type-safe upcast to a wider ref type within the Wasm
    /// bindgen generics type system.
    ///
    /// This enables proper nested conversions that obey subtyping rules,
    /// supporting strict API type checking.
    ///
    /// The common pattern when passing a narrow type is to call `upcast()`
    /// or `upcast_into()` to obtain the correct type for the function usage,
    /// while ensuring safe type checked usage.
    ///
    /// For example, if passing `Promise<Number>` as an argument to a function
    /// where `Promise<JsValue>` is expected, or `Function<JsValue>` as an
    /// argument where `Function<Number>` is expected.
    ///
    /// This is a compile time conversion only by the nature of the erasable
    /// generics type system.
    #[inline]
    fn upcast(&self) -> &T
    where
        Self: ErasableGeneric,
        T: Sized + ErasableGeneric<Repr = <Self as ErasableGeneric>::Repr>,
    {
        unsafe { &*(self as *const Self as *const T) }
    }

    /// Perform a zero-cost type-safe upcast to a wider type within the Wasm
    /// bindgen generics type system.
    ///
    /// This enables proper nested conversions that obey subtyping rules,
    /// supporting strict API type checking.
    ///
    /// The common pattern when passing a narrow type is to call `upcast()`
    /// or `upcast_into()` to obtain the correct type for the function usage,
    /// while ensuring safe type checked usage.
    ///
    /// For example, if passing `Promise<Number>` as an argument to a function
    /// where `Promise<JsValue>` is expected, or `FunctionArgs<JsValue>` as an
    /// argument where `FunctionArgs<Number>` is expected.
    ///
    /// This is a compile time conversion only by the nature of the erasable
    /// generics type system.
    #[inline]
    fn upcast_into(self) -> T
    where
        Self: Sized + ErasableGeneric,
        T: Sized + ErasableGeneric<Repr = <Self as ErasableGeneric>::Repr>,
    {
        unsafe { core::mem::transmute_copy(&core::mem::ManuallyDrop::new(self)) }
    }
}

// Blanket impl: UpcastFrom<S> for T implies Upcast<T> for S
impl<S, T> Upcast<T> for S
where
    T: UpcastFrom<S> + ?Sized,
    S: ?Sized,
{
}

// Reference impls using UpcastFrom
impl<'a, T, Target> UpcastFrom<&'a mut T> for &'a mut Target where Target: UpcastFrom<T> {}
impl<'a, T, Target> UpcastFrom<&'a T> for &'a Target where Target: UpcastFrom<T> {}

// Tuple upcasts with structural covariance
macro_rules! impl_tuple_upcast {
    ([$($T:ident)+] [$($Target:ident)+]) => {
        // Structural covariance: (T...) -> (Target...)
        impl<$($T,)+ $($Target,)+> UpcastFrom<($($T,)+)> for ($($Target,)+)
        where
            $($Target: JsGeneric + UpcastFrom<$T>,)+
            $($T: JsGeneric,)+
        {
        }
        impl<$($T: JsGeneric,)+ $($Target: JsGeneric,)+> UpcastFrom<($($T,)+)> for JsOption<($($Target,)+)>
        where
            $($Target: JsGeneric + UpcastFrom<$T>,)+
            $($T: JsGeneric,)+
        {
        }
    };
}
impl_tuple_upcast!([T1][Target1]);
impl_tuple_upcast!([T1 T2] [Target1 Target2]);
impl_tuple_upcast!([T1 T2 T3] [Target1 Target2 Target3]);
impl_tuple_upcast!([T1 T2 T3 T4] [Target1 Target2 Target3 Target4]);
impl_tuple_upcast!([T1 T2 T3 T4 T5] [Target1 Target2 Target3 Target4 Target5]);
impl_tuple_upcast!([T1 T2 T3 T4 T5 T6] [Target1 Target2 Target3 Target4 Target5 Target6]);
impl_tuple_upcast!([T1 T2 T3 T4 T5 T6 T7] [Target1 Target2 Target3 Target4 Target5 Target6 Target7]);
impl_tuple_upcast!([T1 T2 T3 T4 T5 T6 T7 T8] [Target1 Target2 Target3 Target4 Target5 Target6 Target7 Target8]);

/// A convenience trait for types that erase to [`JsValue`].
///
/// This is a shorthand for `ErasableGeneric<Repr = JsValue>`, used as a bound
/// on generic parameters that must be representable as JavaScript values.
///
/// # When to Use
///
/// Use `JsGeneric` as a trait bound when you need a generic type that:
/// - Can be passed to/from JavaScript
/// - Is type-erased to `JsValue` at the FFI boundary
///
/// # Examples
///
/// ```ignore
/// use wasm_bindgen::JsGeneric;
///
/// fn process_js_values<T: JsGeneric>(items: &[T]) {
///     // T can be any JS-compatible type
/// }
/// ```
///
/// # Implementors
///
/// This trait is automatically implemented for all types that implement
/// `ErasableGeneric<Repr = JsValue>`, including:
/// - All `js_sys` types (`Object`, `Array`, `Function`, etc.)
/// - `JsValue` itself
/// - Custom types imported via `#[wasm_bindgen]`
pub trait JsGeneric:
    ErasableGeneric<Repr = JsValue>
    + UpcastFrom<Self>
    + Upcast<Self>
    + Upcast<JsValue>
    + JsCast
    + 'static
{
}

impl<T: ErasableGeneric<Repr = JsValue> + UpcastFrom<T> + Upcast<JsValue> + JsCast + 'static>
    JsGeneric for T
{
}

/// Value conversion from a type into its canonical [`JsGeneric`] form.
///
/// This trait allows types to be converted into JsGeneric supported types, which
/// are required to be erasably generic with JsValue.
///
/// The single associated type — rather than a free type parameter bounded by
/// `AsRef<T>` — is what makes collection-style APIs infer annotation-free.
/// Given an input `A`, there is exactly one `A::JsCanon`, so rustc never has
/// to search across multiple `AsRef` impls to pick a target element type.
///
/// # Implementations
///
/// Provided impls:
/// - [`JsValue`] in this crate.
/// - Every `#[wasm_bindgen]`-imported type (identity — emitted by the macro).
/// - Every generic `js_sys` container (`Array<T>`, `Promise<T>`, `Set<T>`, …)
///   provides its own identity impl owned by `js_sys`.
/// - References to cloneable implementors, so borrowed iteration can still
///   produce owned JS-generic values.
///
/// This trait is deliberately *not* blanket-implemented over all [`JsGeneric`]
/// types: each implementor explicitly opts in, which leaves room for future
/// wrapper types to pick a non-identity [`Self::JsCanon`].
///
/// # Example
///
/// ```ignore
/// use js_sys::{Array, Number};
///
/// let arr: Array<Number> = (0..10).map(Number::from).collect();
/// ```
pub trait IntoJsGeneric {
    /// The canonical [`JsGeneric`] form of this type.
    type JsCanon: JsGeneric;

    /// Produce the canonical [`JsGeneric`] value for `self`.
    fn to_js(self) -> Self::JsCanon;
}

impl IntoJsGeneric for JsValue {
    type JsCanon = JsValue;
    #[inline]
    fn to_js(self) -> JsValue {
        self
    }
}

// Reference iteration clones the borrowed wrapper to produce an owned value,
// then delegates to that type's canonical conversion.
impl<T: IntoJsGeneric + Clone> IntoJsGeneric for &T {
    type JsCanon = T::JsCanon;
    #[inline]
    fn to_js(self) -> T::JsCanon {
        self.clone().to_js()
    }
}

// Intentionally not a blanket `impl<T: JsGeneric> IntoJsGeneric for T`:
// that would lock in identity for every current and future `JsGeneric` type
// and prevent wrapper types from canonicalising to a different target.
// Instead, implementations are provided explicitly by each owning crate
// (macro-generated for user types; hand-written for `js_sys` containers).

/// Conversion of a `#[wasm_bindgen(catch)]` import's return value, resolved
/// by the type system instead of the macro parsing a literal `Result<...>`
/// out of the written return type (which broke type aliases).
///
/// `Ok` drives the JS `Promise<Ok>` type of `async` catch imports and the
/// descriptor (a catch import is described by its unwrapped success type;
/// the thrown branch travels out-of-band via the exception store).
///
/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
#[cfg_attr(
    wbg_diagnostic,
    diagnostic::on_unimplemented(
        message = "`#[wasm_bindgen(catch)]` imports must return `Result<T, E>` where `T` is a supported by-value type and `E: From<JsValue>`",
        label = "this `catch` import does not return a supported `Result`",
    )
)]
pub trait CatchFromWasmAbi: Sized {
    /// The unwrapped success type.
    type Ok;

    /// The Wasm ABI type the import shim returns (the success type's ABI).
    type Abi: WasmAbi;

    /// Convert a synchronous `catch` import's return: check the exception
    /// store *first* (the abi value is garbage if the import threw), then
    /// convert the success value.
    ///
    /// # Safety
    ///
    /// `abi` must be the value returned by the import shim this call
    /// directly follows, with the exception store unread in between.
    unsafe fn catch_from_abi(abi: Self::Abi) -> Self;

    /// Convert an `async` `catch` import's settled `Promise` result (the
    /// `.await` itself stays in the generated code).
    fn from_js_result(result: Result<Self::Ok, JsValue>) -> Self;

    /// Emit the descriptor for the unwrapped success type.
    fn describe_ok();
}

impl<T, E: From<JsValue>> CatchFromWasmAbi for Result<T, E>
where
    T: crate::convert::ArgAbi<crate::convert::CallScoped, Guard = Option<T>> + WasmDescribe,
{
    type Ok = T;
    type Abi = <T as crate::convert::ArgAbi<crate::convert::CallScoped>>::Abi;

    #[inline]
    unsafe fn catch_from_abi(abi: Self::Abi) -> Self {
        crate::__rt::take_last_exception()?;
        Ok(<T as crate::convert::ArgAbi<crate::convert::CallScoped>>::arg_from_abi(abi).unwrap())
    }

    #[inline]
    fn from_js_result(result: Result<T, JsValue>) -> Self {
        result.map_err(E::from)
    }

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_ok() {
        <T as WasmDescribe>::describe();
    }
}
