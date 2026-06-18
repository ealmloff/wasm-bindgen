//! Export argument ABI.
//!
//! This mirrors `FromWasmAbi`, `RefFromWasmAbi`, `LongRefFromWasmAbi`, and
//! `RefMutFromWasmAbi`, but is implemented for the argument type itself. That
//! lets the macro use trait resolution instead of inspecting syntax, so aliases
//! like `type StrRef<'a> = &'a str` follow the same path as `&str`.
//!
//! # Unstable
//!
//! This is part of the internal [`convert`](crate::convert) module, **no
//! stability guarantees** are provided. Use at your own risk. See its
//! documentation for more details.

use alloc::boxed::Box;
use core::borrow::Borrow;
use core::mem::{self, ManuallyDrop};
use core::ops::{Deref, DerefMut};

use crate::convert::slices::WasmSlice;
use crate::convert::{FromWasmAbi, LongRefFromWasmAbi, RefFromWasmAbi, RefMutFromWasmAbi, WasmAbi};
use crate::describe::{inform, WasmDescribe, LONGREF, REF, REFMUT};
use crate::JsValue;

mod sealed {
    pub trait Sealed {}
}

/// How long a borrowed argument must remain valid.
pub trait BorrowScope: sealed::Sealed {}

/// The borrow only needs to outlive the synchronous export call.
pub struct CallScoped;

/// The borrow must outlive the `Future` an `async` export returns.
pub struct Anchored;

impl sealed::Sealed for CallScoped {}
impl sealed::Sealed for Anchored {}
impl BorrowScope for CallScoped {}
impl BorrowScope for Anchored {}

/// Decode one exported-function argument from the Wasm ABI, for borrow scope
/// `S`.
///
/// See [`ArgAbiProject`] for projecting the decoded pieces into the Rust
/// argument type.
pub trait ArgAbi<S: BorrowScope> {
    /// The Wasm ABI type this argument crosses the boundary as.
    type Abi: WasmAbi;

    /// The by-value part of the decoded argument.
    type Value;

    /// The retained part that backs a borrowed argument.
    type Anchor;

    /// The type whose `UnwindSafe`-ness gates this argument under
    /// `panic = "unwind"`: the argument itself for a by-value argument, and
    /// `*const T` for `&T`/`&mut T` (`*const T: UnwindSafe` exactly when
    /// `T: RefUnwindSafe`, matching the previous per-shape checks).
    type UnwindCheck: ?Sized;

    /// Describe this argument to the `wasm-bindgen` CLI, handling the
    /// `LONGREF` marker an `async` export's shared borrow needs.
    fn describe_arg();

    /// Decode both argument parts from the incoming ABI value.
    ///
    /// # Safety
    ///
    /// Same as [`FromWasmAbi::from_abi`].
    unsafe fn arg_from_abi(js: Self::Abi) -> (Self::Value, Self::Anchor);
}

/// Project a decoded argument into the exact type the Rust function receives.
///
/// The generated shim owns the anchor and borrows from it for the projected
/// argument lifetime.
pub trait ArgAbiProject<'a, S: BorrowScope>: ArgAbi<S> {
    /// Produce the call argument: move the by-value part out of `value`, or
    /// borrow it from the caller-owned `anchor`.
    fn project(value: Self::Value, anchor: &'a mut Self::Anchor) -> Self;
}

/// Describe a `&T` argument of a synchronous export (`REF` + `T`).
#[doc(hidden)]
#[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
pub fn describe_ref_arg<T: WasmDescribe + ?Sized>() {
    inform(REF);
    T::describe();
}

/// Describe a `&mut T` argument (`REFMUT` + `T`).
#[doc(hidden)]
#[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
pub fn describe_ref_mut_arg<T: WasmDescribe + ?Sized>() {
    inform(REFMUT);
    T::describe();
}

/// Describe a `&T` argument of an `async` export (`LONGREF` + `T`).
#[doc(hidden)]
#[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
pub fn describe_long_ref_arg<T: WasmDescribe + ?Sized>() {
    inform(LONGREF);
    T::describe();
}

impl<S: BorrowScope, T: FromWasmAbi> ArgAbi<S> for T {
    type Abi = <T as FromWasmAbi>::Abi;
    type Value = T;
    type Anchor = ();
    type UnwindCheck = T;

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        T::describe();
    }

    unsafe fn arg_from_abi(js: Self::Abi) -> (Self::Value, Self::Anchor) {
        (T::from_abi(js), ())
    }
}

impl<'a, S: BorrowScope, T: FromWasmAbi> ArgAbiProject<'a, S> for T {
    fn project(value: Self::Value, _anchor: &'a mut Self::Anchor) -> Self {
        value
    }
}

impl ArgAbi<CallScoped> for &str {
    type Abi = WasmSlice;
    type Value = ();
    type Anchor = Box<str>;
    type UnwindCheck = *const str;

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        describe_ref_arg::<str>();
    }

    unsafe fn arg_from_abi(js: Self::Abi) -> (Self::Value, Self::Anchor) {
        (
            (),
            mem::transmute::<Box<[u8]>, Box<str>>(<Box<[u8]>>::from_abi(js)),
        )
    }
}

impl<'a> ArgAbiProject<'a, CallScoped> for &'a str {
    fn project(_value: Self::Value, anchor: &'a mut Self::Anchor) -> Self {
        anchor
    }
}

impl ArgAbi<Anchored> for &str {
    type Abi = WasmSlice;
    type Value = ();
    type Anchor = Box<str>;
    type UnwindCheck = *const str;

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        describe_long_ref_arg::<str>();
    }

    unsafe fn arg_from_abi(js: Self::Abi) -> (Self::Value, Self::Anchor) {
        <&str as ArgAbi<CallScoped>>::arg_from_abi(js)
    }
}

impl<'a> ArgAbiProject<'a, Anchored> for &'a str {
    fn project(_value: Self::Value, anchor: &'a mut Self::Anchor) -> Self {
        <Box<str> as Borrow<str>>::borrow(anchor)
    }
}

impl ArgAbi<CallScoped> for &JsValue {
    type Abi = u32;
    type Value = ();
    type Anchor = ManuallyDrop<JsValue>;
    type UnwindCheck = *const JsValue;

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        describe_ref_arg::<JsValue>();
    }

    unsafe fn arg_from_abi(js: Self::Abi) -> (Self::Value, Self::Anchor) {
        ((), ManuallyDrop::new(JsValue::_new(js)))
    }
}

impl<'a> ArgAbiProject<'a, CallScoped> for &'a JsValue {
    fn project(_value: Self::Value, anchor: &'a mut Self::Anchor) -> Self {
        anchor
    }
}

impl ArgAbi<Anchored> for &JsValue {
    type Abi = u32;
    type Value = ();
    type Anchor = JsValue;
    type UnwindCheck = *const JsValue;

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        describe_long_ref_arg::<JsValue>();
    }

    unsafe fn arg_from_abi(js: Self::Abi) -> (Self::Value, Self::Anchor) {
        ((), JsValue::_new(js))
    }
}

impl<'a> ArgAbiProject<'a, Anchored> for &'a JsValue {
    fn project(_value: Self::Value, anchor: &'a mut Self::Anchor) -> Self {
        anchor
    }
}

impl<T> RefFromWasmAbi for T
where
    T: ?Sized + WasmDescribe + 'static,
    &'static T: ArgAbi<CallScoped>,
    <&'static T as ArgAbi<CallScoped>>::Anchor: Deref<Target = T>,
{
    type Abi = <&'static T as ArgAbi<CallScoped>>::Abi;
    type Anchor = <&'static T as ArgAbi<CallScoped>>::Anchor;

    unsafe fn ref_from_abi(js: Self::Abi) -> Self::Anchor {
        <&'static T as ArgAbi<CallScoped>>::arg_from_abi(js).1
    }
}

impl<T> RefMutFromWasmAbi for T
where
    T: ?Sized + WasmDescribe + 'static,
    &'static mut T: ArgAbi<CallScoped>,
    <&'static mut T as ArgAbi<CallScoped>>::Anchor: DerefMut<Target = T>,
{
    type Abi = <&'static mut T as ArgAbi<CallScoped>>::Abi;
    type Anchor = <&'static mut T as ArgAbi<CallScoped>>::Anchor;

    unsafe fn ref_mut_from_abi(js: Self::Abi) -> Self::Anchor {
        <&'static mut T as ArgAbi<CallScoped>>::arg_from_abi(js).1
    }
}

impl<T> LongRefFromWasmAbi for T
where
    T: ?Sized + WasmDescribe + 'static,
    &'static T: ArgAbi<Anchored>,
    <&'static T as ArgAbi<Anchored>>::Anchor: Borrow<T>,
{
    type Abi = <&'static T as ArgAbi<Anchored>>::Abi;
    type Anchor = <&'static T as ArgAbi<Anchored>>::Anchor;

    unsafe fn long_ref_from_abi(js: Self::Abi) -> Self::Anchor {
        <&'static T as ArgAbi<Anchored>>::arg_from_abi(js).1
    }
}
