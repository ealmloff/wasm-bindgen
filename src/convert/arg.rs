//! # ⚠️ Unstable
//!
//! This is an internal module, no stability guarantees are provided. Use at
//! your own risk.
//!
//! A single trait for converting an exported function's argument from its
//! ABI form, resolved by the type system instead of by the macro inspecting
//! type *syntax*. This is what makes type aliases behave identically to the
//! types they alias: trait resolution sees through aliases, a syntactic
//! `match` on `syn::Type` does not.
//!
//! `ArgAbi<S>` is the single JS→Rust conversion trait, replacing the
//! removed per-shape family (`FromWasmAbi`, `OptionFromWasmAbi`,
//! `RefFromWasmAbi`, `LongRefFromWasmAbi`, `RefMutFromWasmAbi`): the macro
//! only ever names the *written* type against `ArgAbi<S>`. Owned, `&T`,
//! `&mut T`, and `Option<T>` are all plain impls — here, in `slices.rs` /
//! `impls.rs`, and in the impls `#[wasm_bindgen]` emits for exported and
//! imported types.
//!
//! The scope parameter `S` carries the one piece of context that genuinely
//! changes conversion behavior and that the macro *can* know without
//! inspecting types: whether the borrow only needs to live for a
//! synchronous call ([`CallScoped`]) or must be anchored across `.await`
//! points in the future of an `async` export ([`Anchored`]).
//!
//! `Option<T>` support is part of the trait protocol rather than a helper
//! trait: the orphan rules forbid downstream crates from implementing
//! anything for `Option<TheirType>`, so each element type carries its own
//! `None` encoding ([`ArgAbi::OptionSupport`] / [`ArgAbi::OptionAbi`] /
//! [`ArgAbi::option_arg_from_abi`]) and a single generic `Option<T>` impl
//! below dispatches through it.

use alloc::boxed::Box;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

use crate::convert::{VectorFromWasmAbi, WasmAbi};
use crate::describe::{WasmDescribe, LONGREF, REF};
use crate::JsValue;

mod private {
    pub trait Sealed {}
    impl Sealed for super::CallScoped {}
    impl Sealed for super::Anchored {}
}

/// The borrow scope of an exported function call: how long argument guards
/// must keep their projections alive.
pub trait Scope: private::Sealed {
    /// The descriptor tag for a shared reference under this scope —
    /// [`REF`] for [`CallScoped`], [`LONGREF`] for [`Anchored`].
    const SHARED_REF: u32;
}

/// Arguments live exactly for the duration of a synchronous export call.
pub struct CallScoped;

/// Arguments must be anchored by owned guards that live inside the future
/// of an `async` export, across `.await` points.
pub struct Anchored;

impl Scope for CallScoped {
    const SHARED_REF: u32 = REF;
}

impl Scope for Anchored {
    const SHARED_REF: u32 = LONGREF;
}

/// Conversion of a single exported-function argument from its ABI form,
/// under borrow scope `S`.
///
/// `Guard` owns whatever keeps `Projected` alive (the decoded value itself
/// for owned arguments, an anchor like `Box<str>` or `RcRef<Class>` for
/// references); the generated code keeps every guard alive until the call
/// (or the `async` export's future) completes, then drops them in reverse
/// declaration order — exactly the lifetime the previous per-shape codegen
/// gave its conversion locals. Side effects on guard drop (e.g. the mutable
/// slice write-back in [`MutSlice`](crate::convert::slices) `Drop`) keep
/// their timing.
#[cfg_attr(
    wbg_diagnostic,
    diagnostic::on_unimplemented(
        message = "`{Self}` is not supported as an argument of an exported function",
        label = "unsupported `#[wasm_bindgen]` argument type",
        note = "arguments are supported for primitives, owned JS values, `Option`s of supported types, and references like `&str`, slices of supported primitives, `&JsValue`, exported structs, and imported types",
    )
)]
pub trait ArgAbi<S: Scope> {
    /// The wire type crossing the wasm boundary for this argument.
    type Abi: WasmAbi;

    /// Owns whatever keeps the projected argument valid for the call, and
    /// knows how to project it ([`ArgGuard`]).
    type Guard: ArgGuard;

    /// Which unwind-safety property export shims assert for this written
    /// argument type under `panic = "unwind"`:
    /// [`OwnedCheck`](crate::__rt::marker::OwnedCheck)`<T>` requires
    /// `T: UnwindSafe`, [`RefCheck`](crate::__rt::marker::RefCheck)`<P>`
    /// requires the pointee `P: RefUnwindSafe`. Checked only by
    /// `__rt::ensure_arg_unwind_safe` at export call sites — defining or
    /// returning a non-unwind-safe type stays legal.
    type UnwindCheck;

    /// Decode the guard from the incoming ABI value.
    ///
    /// # Safety
    ///
    /// `abi` must be a valid ABI value produced by the corresponding JS
    /// glue for this argument type, and must not be used again.
    unsafe fn arg_from_abi(abi: Self::Abi) -> Self::Guard;

    /// Emit this argument's descriptor bytes — byte-identical to what the
    /// previous syntactic dispatch emitted for the same written type.
    fn describe_arg();
}

/// How a decoded argument is handed to the user's function: projection is
/// a property of the *guard*, so it is written once per guard shape below
/// instead of once per argument type.
pub trait ArgGuard {
    /// What the user's function receives, borrowed from the guard.
    type Projected<'a>
    where
        Self: 'a;

    /// Project the argument. Called exactly once per guard.
    fn project(&mut self) -> Self::Projected<'_>;
}

/// Owned arguments: the guard is the decoded value (`Option`, not
/// `ManuallyDrop`, so that if a later argument's conversion or the call
/// unwinds, the already-decoded value drops) and projection hands it over.
impl<T> ArgGuard for Option<T> {
    type Projected<'a>
        = T
    where
        Self: 'a;

    #[inline(always)]
    fn project(&mut self) -> T {
        self.take().unwrap()
    }
}

/// A shared-reference anchor: projection borrows through `Deref` (e.g.
/// `Box<str>` for `&str`, `RcRef<Class>` for `&Class`).
pub struct Shared<G>(pub G);

impl<G: Deref> ArgGuard for Shared<G> {
    type Projected<'a>
        = &'a G::Target
    where
        Self: 'a;

    #[inline(always)]
    fn project(&mut self) -> &G::Target {
        &self.0
    }
}

impl<G: Deref> Deref for Shared<G> {
    type Target = G::Target;
    #[inline(always)]
    fn deref(&self) -> &G::Target {
        &self.0
    }
}

/// A mutable-reference anchor: projection borrows through `DerefMut`
/// (e.g. `MutSlice<T>` for `&mut [T]`, `RcRefMut<Class>` for
/// `&mut Class`).
pub struct Exclusive<G>(pub G);

impl<G: DerefMut> ArgGuard for Exclusive<G> {
    type Projected<'a>
        = &'a mut G::Target
    where
        Self: 'a;

    #[inline(always)]
    fn project(&mut self) -> &mut G::Target {
        &mut self.0
    }
}

/// An anchor that owns the value outright and projects a plain borrow of
/// it — used where the borrow must outlive a synchronous call (the
/// [`Anchored`] scope), e.g. `&JsValue` arguments of `async` exports.
pub struct OwnedAnchor<T>(pub T);

impl<T> ArgGuard for OwnedAnchor<T> {
    type Projected<'a>
        = &'a T
    where
        Self: 'a;

    #[inline(always)]
    fn project(&mut self) -> &T {
        &self.0
    }
}

impl<T> Deref for OwnedAnchor<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}

/// The `None` encoding of an element type whose `Option<Self>` is a
/// supported argument.
///
/// This is a capability of the *element* rather than an impl on
/// `Option<Self>` because the orphan rules forbid downstream crates from
/// implementing anything for `Option<TheirType>`; `#[wasm_bindgen]` emits
/// this impl next to the type, and the single generic `Option<T>` impl
/// below dispatches through it. Types without an `Option` encoding simply
/// don't implement it.
#[cfg_attr(
    wbg_diagnostic,
    diagnostic::on_unimplemented(
        message = "`Option<{Self}>` is not supported as an argument of an exported function",
        label = "`Option` of this type has no ABI encoding",
    )
)]
pub trait OptionArgAbi<S: Scope>: Sized {
    /// The wire type for `Option<Self>` (not necessarily the element's
    /// `Abi`: e.g. `u32` rides the JS number ABI as `f64` when optional).
    type OptionAbi: WasmAbi;

    /// Decode `Option<Self>` from its wire form.
    ///
    /// # Safety
    ///
    /// Same contract as [`ArgAbi::arg_from_abi`].
    unsafe fn option_arg_from_abi(abi: Self::OptionAbi) -> Option<Self>;
}

/// The single `Option<T>` argument impl: `T` carries its own `None`
/// encoding in its [`OptionArgAbi`] impl.
impl<S: Scope, T> ArgAbi<S> for Option<T>
where
    T: OptionArgAbi<S> + WasmDescribe,
{
    type Abi = <T as OptionArgAbi<S>>::OptionAbi;
    type Guard = Option<Option<T>>;
    // The whole `Option<T>` is consumed by value, just like any other
    // owned argument (`Option<T>: UnwindSafe ⇔ T: UnwindSafe`).
    type UnwindCheck = crate::__rt::marker::OwnedCheck<Option<T>>;

    #[inline(always)]
    unsafe fn arg_from_abi(abi: Self::Abi) -> Self::Guard {
        Some(<T as OptionArgAbi<S>>::option_arg_from_abi(abi))
    }

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        <Option<T> as WasmDescribe>::describe();
    }
}

/// Implements `ArgAbi<S>` for a by-value ("owned") type: the guard is the
/// decoded value itself (`Option`, not `ManuallyDrop`, so that if a later
/// argument's conversion or the call unwinds, the already-decoded value
/// drops), and projection hands it over.
///
/// The second form also implements `ArgAbi<S>` for `Option<$t>` with an
/// in-band `None` sentinel sharing `$t`'s ABI — the wire format previously
/// expressed through a separate helper trait.
///
/// Crate-internal: the proc macro emits the equivalent impls directly
/// (`by_value_arg_abi_impls` in macro-support's codegen) because a macro
/// invocation in module-item position stalls rustc's import resolution
/// under `use crate::wasm_bindgen;` plus glob re-exports (issue #4597).
macro_rules! by_value_arg_abi {
    // `Option<$t>` shares `$t`'s ABI with an in-band `None` sentinel.
    (impl$(<$($g:ident),*>)? ArgAbi for $t:ty $(where ($($wc:tt)*))? {
        type Abi = $abi:ty; |$js:ident| $decode:expr
     } with Option (is_none = $is_none:expr)) => {
        $crate::convert::arg::by_value_arg_abi!(
            impl$(<$($g),*>)? ArgAbi for $t $(where ($($wc)*))? { type Abi = $abi; |$js| $decode }
            with Option (type OptionAbi = $abi; is_none = $is_none)
        );
    };
    // `Option<$t>` rides a different wire type, still sentinel-encoded.
    (impl$(<$($g:ident),*>)? ArgAbi for $t:ty $(where ($($wc:tt)*))? {
        type Abi = $abi:ty; |$js:ident| $decode:expr
     } with Option (type OptionAbi = $opt_abi:ty; is_none = $is_none:expr)) => {
        $crate::convert::arg::by_value_arg_abi!(
            impl$(<$($g),*>)? ArgAbi for $t $(where ($($wc)*))? { type Abi = $abi; |$js| $decode }
            with Option (
                type OptionAbi = $opt_abi;
                |$js| if $is_none {
                    None
                } else {
                    #[allow(unused_unsafe, clippy::redundant_closure_call)]
                    Some((|| -> $t { unsafe { $decode } })())
                }
            )
        );
    };
    // `Option<$t>` with a fully custom decode.
    (impl$(<$($g:ident),*>)? ArgAbi for $t:ty $(where ($($wc:tt)*))? {
        type Abi = $abi:ty; |$js:ident| $decode:expr
     } with Option (type OptionAbi = $opt_abi:ty; |$opt_js:ident| $opt_decode:expr)) => {
        $crate::convert::arg::by_value_arg_abi!(
            impl$(<$($g),*>)? ArgAbi for $t $(where ($($wc)*))? { type Abi = $abi; |$js| $decode }
        );

        impl<WbgS: $crate::convert::Scope $(, $($g),*)?> $crate::convert::OptionArgAbi<WbgS> for $t
        where
            $($($wc)*)?
        {
            type OptionAbi = $opt_abi;

            #[inline(always)]
            #[allow(unreachable_code, clippy::diverging_sub_expression)]
            unsafe fn option_arg_from_abi($opt_js: Self::OptionAbi) -> Option<Self> {
                $opt_decode
            }
        }
    };
    (impl$(<$($g:ident),*>)? ArgAbi for $t:ty $(where ($($wc:tt)*))? {
        type Abi = $abi:ty; |$js:ident| $decode:expr
     }) => {
        impl<WbgS: $crate::convert::Scope $(, $($g),*)?> $crate::convert::ArgAbi<WbgS> for $t
        where
            $($($wc)*)?
        {
            type Abi = $abi;
            type Guard = Option<$t>;
            type UnwindCheck = $crate::__rt::marker::OwnedCheck<$t>;

            #[inline(always)]
            #[allow(unreachable_code, clippy::diverging_sub_expression)]
            unsafe fn arg_from_abi($js: Self::Abi) -> Self::Guard {
                // The closure keeps any `return`s inside the decode
                // expression scoped to the decode itself.
                #[allow(unused_unsafe, clippy::redundant_closure_call)]
                Some((|| -> $t { unsafe { $decode } })())
            }

            #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
            fn describe_arg() {
                <$t as $crate::describe::WasmDescribe>::describe();
            }
        }
    };
}
pub(crate) use by_value_arg_abi;

/// Implements `ArgAbi<S>` for a reference type: the guard owns the decoded
/// data (an anchor) and projection borrows from it.
///
/// The first form is scope-generic for types whose anchor is valid under
/// either scope, described as `REF`/`LONGREF` picked by the scope; the
/// `mut` form is likewise scope-generic and described as `REFMUT`; the
/// last form is for a single fixed scope with an explicit descriptor tag.
///
/// Crate-internal: the proc macro emits the equivalent impls directly
/// (`borrowed_arg_abi_impl` in macro-support's codegen) — see
/// [`by_value_arg_abi`] for why.
macro_rules! borrowed_arg_abi {
    (impl$(($($g:tt)*))? ArgAbi for &$pointee:ty $(where ($($wc:tt)*))? {
        type Abi = $abi:ty;
        type Guard = $guard:ty;
        |$js:ident| $decode:expr;
        describe = shared_ref($elem:ty);
    }) => {
        impl<$($($g)*,)? WbgS: $crate::convert::Scope> $crate::convert::ArgAbi<WbgS>
            for &$pointee
        where
            $($($wc)*)?
        {
            type Abi = $abi;
            type Guard = $guard;
            type UnwindCheck = $crate::__rt::marker::RefCheck<$pointee>;

            #[inline(always)]
            unsafe fn arg_from_abi($js: Self::Abi) -> Self::Guard {
                $decode
            }

            #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
            fn describe_arg() {
                $crate::describe::inform(WbgS::SHARED_REF);
                <$elem as $crate::describe::WasmDescribe>::describe();
            }
        }
    };
    (impl$(($($g:tt)*))? ArgAbi for &mut $pointee:ty $(where ($($wc:tt)*))? {
        type Abi = $abi:ty;
        type Guard = $guard:ty;
        |$js:ident| $decode:expr;
        describe = refmut($elem:ty);
    }) => {
        impl<$($($g)*,)? WbgS: $crate::convert::Scope> $crate::convert::ArgAbi<WbgS>
            for &mut $pointee
        where
            $($($wc)*)?
        {
            type Abi = $abi;
            type Guard = $guard;
            type UnwindCheck = $crate::__rt::marker::RefCheck<$pointee>;

            #[inline(always)]
            unsafe fn arg_from_abi($js: Self::Abi) -> Self::Guard {
                $decode
            }

            #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
            fn describe_arg() {
                $crate::describe::inform($crate::describe::REFMUT);
                <$elem as $crate::describe::WasmDescribe>::describe();
            }
        }
    };
    (impl$(($($g:tt)*))? ArgAbi<$scope:ty> for &$pointee:ty $(where ($($wc:tt)*))? {
        type Abi = $abi:ty;
        type Guard = $guard:ty;
        |$js:ident| $decode:expr;
        describe = $tag:ident($elem:ty);
    }) => {
        impl$(<$($g)*>)? $crate::convert::ArgAbi<$scope> for &$pointee
        where
            $($($wc)*)?
        {
            type Abi = $abi;
            type Guard = $guard;
            type UnwindCheck = $crate::__rt::marker::RefCheck<$pointee>;

            #[inline(always)]
            unsafe fn arg_from_abi($js: Self::Abi) -> Self::Guard {
                $decode
            }

            #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
            fn describe_arg() {
                $crate::describe::inform($crate::describe::$tag);
                <$elem as $crate::describe::WasmDescribe>::describe();
            }
        }
    };
}
pub(crate) use borrowed_arg_abi;

// `&str`: the guard owns the UTF-8 copy under both scopes; the JS glue
// always passes valid UTF-8.
borrowed_arg_abi!(
    impl ArgAbi for &str {
        type Abi = <u8 as VectorFromWasmAbi>::Abi;
        type Guard = Shared<Box<str>>;
        |abi| Shared(core::mem::transmute::<Box<[u8]>, Box<str>>(
            <u8 as VectorFromWasmAbi>::vector_from_abi(abi),
        ));
        describe = shared_ref(str);
    }
);

// `&JsValue` in a synchronous export: the JS glue keeps the heap value
// alive for the duration of the call, so the guard is a borrowed handle
// that must not be freed (`ManuallyDrop`).
borrowed_arg_abi!(
    impl ArgAbi<CallScoped> for &JsValue {
        type Abi = u32;
        type Guard = Shared<ManuallyDrop<JsValue>>;
        |abi| Shared(ManuallyDrop::new(JsValue::_new(abi)));
        describe = REF(JsValue);
    }
);

// `&JsValue` in an `async` export: the borrow must outlive the
// synchronous call, so the guard owns the value (its own slot in the JS
// heap) and frees it when the future completes.
borrowed_arg_abi!(
    impl ArgAbi<Anchored> for &JsValue {
        type Abi = u32;
        type Guard = OwnedAnchor<JsValue>;
        |abi| OwnedAnchor(JsValue::_new(abi));
        describe = LONGREF(JsValue);
    }
);
