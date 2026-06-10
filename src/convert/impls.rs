use alloc::boxed::Box;
use alloc::vec::Vec;
use core::char;
use core::mem::{self, ManuallyDrop};
use core::ptr::NonNull;

use crate::__rt::marker::ErasableGeneric;
use crate::__rt::{WasmSignedWordRepr, WasmWordRepr};
use crate::convert::arg::by_value_arg_abi;
use crate::convert::traits::{WasmAbi, WasmPrimitive};
use crate::convert::{
    ArgAbi, IntoWasmAbi, OptionIntoWasmAbi, ReturnWasmAbi, Scope, TryFromJsValue, UpcastFrom,
    VectorFromWasmAbi,
};
use crate::describe::WasmDescribe;
use crate::sys::Promising;
use crate::sys::{JsOption, Undefined};
use crate::{Clamped, JsError, JsValue, UnwrapThrowExt};

// Primitive types can always be passed over the ABI.
impl<T: WasmPrimitive> WasmAbi for T {
    type Prim1 = Self;
    type Prim2 = ();
    type Prim3 = ();
    type Prim4 = ();

    #[inline]
    fn split(self) -> (Self, (), (), ()) {
        (self, (), (), ())
    }

    #[inline]
    fn join(prim: Self, _: (), _: (), _: ()) -> Self {
        prim
    }
}

impl WasmAbi for i128 {
    type Prim1 = u64;
    type Prim2 = u64;
    type Prim3 = ();
    type Prim4 = ();

    #[inline]
    fn split(self) -> (u64, u64, (), ()) {
        let low = self as u64;
        let high = (self >> 64) as u64;
        (low, high, (), ())
    }

    #[inline]
    fn join(low: u64, high: u64, _: (), _: ()) -> Self {
        (((high as u128) << 64) | low as u128) as i128
    }
}
impl WasmAbi for u128 {
    type Prim1 = u64;
    type Prim2 = u64;
    type Prim3 = ();
    type Prim4 = ();

    #[inline]
    fn split(self) -> (u64, u64, (), ()) {
        let low = self as u64;
        let high = (self >> 64) as u64;
        (low, high, (), ())
    }

    #[inline]
    fn join(low: u64, high: u64, _: (), _: ()) -> Self {
        ((high as u128) << 64) | low as u128
    }
}

impl<T: WasmAbi<Prim4 = ()>> WasmAbi for Option<T> {
    /// Whether this `Option` is a `Some` value.
    type Prim1 = u32;
    type Prim2 = T::Prim1;
    type Prim3 = T::Prim2;
    type Prim4 = T::Prim3;

    #[inline]
    fn split(self) -> (u32, T::Prim1, T::Prim2, T::Prim3) {
        match self {
            None => (
                0,
                Default::default(),
                Default::default(),
                Default::default(),
            ),
            Some(value) => {
                let (prim1, prim2, prim3, ()) = value.split();
                (1, prim1, prim2, prim3)
            }
        }
    }

    #[inline]
    fn join(is_some: u32, prim1: T::Prim1, prim2: T::Prim2, prim3: T::Prim3) -> Self {
        if is_some == 0 {
            None
        } else {
            Some(T::join(prim1, prim2, prim3, ()))
        }
    }
}

macro_rules! type_wasm_native {
    ($($t:tt as $c:tt)*) => ($(
        impl IntoWasmAbi for $t {
            type Abi = $c;

            #[inline]
            fn into_abi(self) -> $c { self as $c }
        }

        by_value_arg_abi!(
            impl ArgAbi for $t { type Abi = $c; |js| js as $t }
            with Option (type OptionAbi = Option<$c>; |js| js.map(|v: $c| v as $t))
        );

        impl IntoWasmAbi for Option<$t> {
            type Abi = Option<$c>;

            #[inline]
            fn into_abi(self) -> Self::Abi {
                self.map(|v| v as $c)
            }
        }

        impl UpcastFrom<$t> for JsValue {}
        impl UpcastFrom<$t> for JsOption<JsValue> {}
        impl UpcastFrom<$t> for $t {}
    )*)
}

type_wasm_native!(
    i64 as i64
    u64 as u64
    i128 as i128
    u128 as u128
    f64 as f64
);

impl UpcastFrom<u64> for u128 {}
impl UpcastFrom<u64> for JsOption<u128> {}
impl UpcastFrom<i64> for i128 {}
impl UpcastFrom<i64> for JsOption<i128> {}

/// Sentinel value used to encode `None` for optional pointer-sized and
/// 32-bit numeric values transferred over the JS `number` ABI.
///
/// `2^53 - 1` (`Number.MAX_SAFE_INTEGER`) is chosen because it is:
/// - exactly representable as an `f64` (so JS round-trips it losslessly),
/// - outside the range of any valid `i32`/`u32`/`f32` value (so it can't
///   collide with a real `Some(...)` payload from those types), and
/// - far above any plausible wasm64 pointer (which is bounded by the
///   memory-64 address space limit, well below `2^53`).
const F64_ABI_OPTION_SENTINEL: f64 = 9007199254740991_f64;

macro_rules! type_wasm_native_f64_option {
    ($($t:tt as $c:tt)*) => ($(
        impl IntoWasmAbi for $t {
            type Abi = $c;

            #[inline]
            fn into_abi(self) -> $c { self as $c }
        }

        by_value_arg_abi!(
            impl ArgAbi for $t { type Abi = $c; |js| js as $t }
            with Option (type OptionAbi = f64; |js| if js == F64_ABI_OPTION_SENTINEL {
                None
            } else {
                Some(js as $c as $t)
            })
        );

        unsafe impl ErasableGeneric for $t {
            type Repr = $t;
        }

        impl Promising for $t {
            type Resolution = $t;
        }

        impl IntoWasmAbi for Option<$t> {
            type Abi = f64;

            #[inline]
            fn into_abi(self) -> Self::Abi {
                self.map(|v| v as $c as f64).unwrap_or(F64_ABI_OPTION_SENTINEL)
            }
        }


        impl UpcastFrom<$t> for JsValue {}
        impl UpcastFrom<$t> for JsOption<JsValue> {}
        impl UpcastFrom<$t> for $t {}
    )*)
}

type_wasm_native_f64_option!(
    i32 as i32
    u32 as u32
    f32 as f32
    isize as WasmSignedWordRepr
    usize as WasmWordRepr
);

#[cfg(target_pointer_width = "32")]
impl UpcastFrom<isize> for i32 {}
#[cfg(target_pointer_width = "32")]
impl UpcastFrom<isize> for JsOption<i32> {}

impl UpcastFrom<isize> for i64 {}
impl UpcastFrom<isize> for JsOption<i64> {}
impl UpcastFrom<isize> for i128 {}
impl UpcastFrom<isize> for JsOption<i128> {}

impl UpcastFrom<i32> for isize {}
impl UpcastFrom<i32> for JsOption<isize> {}
impl UpcastFrom<i32> for i64 {}
impl UpcastFrom<i32> for JsOption<i64> {}
impl UpcastFrom<i32> for i128 {}
impl UpcastFrom<i32> for JsOption<i128> {}

impl UpcastFrom<u32> for usize {}
impl UpcastFrom<u32> for JsOption<usize> {}
impl UpcastFrom<u32> for u64 {}
impl UpcastFrom<u32> for JsOption<u64> {}
impl UpcastFrom<u32> for u128 {}
impl UpcastFrom<u32> for JsOption<u128> {}

#[cfg(target_pointer_width = "32")]
impl UpcastFrom<usize> for u32 {}
#[cfg(target_pointer_width = "32")]
impl UpcastFrom<usize> for JsOption<u32> {}
impl UpcastFrom<usize> for u64 {}
impl UpcastFrom<usize> for JsOption<u64> {}
impl UpcastFrom<usize> for u128 {}
impl UpcastFrom<usize> for JsOption<u128> {}

impl UpcastFrom<f32> for f64 {}
impl UpcastFrom<f32> for JsOption<f64> {}

/// The sentinel value is 0xFF_FFFF for primitives with less than 32 bits.
///
/// This value is used, so all small primitive types (`bool`, `i8`, `u8`,
/// `i16`, `u16`, `char`) can use the same JS glue code. `char::MAX` is
/// 0x10_FFFF btw.
const U32_ABI_OPTION_SENTINEL: u32 = 0x00FF_FFFFu32;

macro_rules! type_abi_as_u32 {
    ($($t:tt)*) => ($(
        impl IntoWasmAbi for $t {
            type Abi = u32;

            #[inline]
            fn into_abi(self) -> u32 { self as u32 }
        }

        by_value_arg_abi!(
            impl ArgAbi for $t { type Abi = u32; |js| js as $t }
            with Option (is_none = js == U32_ABI_OPTION_SENTINEL)
        );

        impl OptionIntoWasmAbi for $t {
            #[inline]
            fn none() -> u32 { U32_ABI_OPTION_SENTINEL }
        }

        unsafe impl ErasableGeneric for $t {
            type Repr = $t;
        }

        impl Promising for $t {
            type Resolution = $t;
        }

        impl UpcastFrom<$t> for JsValue {}
        impl UpcastFrom<$t> for JsOption<JsValue> {}
        impl UpcastFrom<$t> for $t {}
    )*)
}

type_abi_as_u32!(i8 u8 i16 u16);

impl UpcastFrom<i8> for i16 {}
impl UpcastFrom<i8> for JsOption<i16> {}
impl UpcastFrom<i8> for i32 {}
impl UpcastFrom<i8> for JsOption<i32> {}
impl UpcastFrom<i8> for i64 {}
impl UpcastFrom<i8> for JsOption<i64> {}
impl UpcastFrom<i8> for i128 {}
impl UpcastFrom<i8> for JsOption<i128> {}

impl UpcastFrom<u8> for u16 {}
impl UpcastFrom<u8> for JsOption<u16> {}
impl UpcastFrom<u8> for u32 {}
impl UpcastFrom<u8> for JsOption<u32> {}
impl UpcastFrom<u8> for u64 {}
impl UpcastFrom<u8> for JsOption<u64> {}
impl UpcastFrom<u8> for u128 {}
impl UpcastFrom<u8> for JsOption<u128> {}

impl UpcastFrom<i16> for i32 {}
impl UpcastFrom<i16> for JsOption<i32> {}
impl UpcastFrom<i16> for i64 {}
impl UpcastFrom<i16> for JsOption<i64> {}
impl UpcastFrom<i16> for i128 {}
impl UpcastFrom<i16> for JsOption<i128> {}

impl UpcastFrom<u16> for u32 {}
impl UpcastFrom<u16> for JsOption<u32> {}
impl UpcastFrom<u16> for u64 {}
impl UpcastFrom<u16> for JsOption<u64> {}
impl UpcastFrom<u16> for u128 {}
impl UpcastFrom<u16> for JsOption<u128> {}

impl IntoWasmAbi for bool {
    type Abi = u32;

    #[inline]
    fn into_abi(self) -> u32 {
        self as u32
    }
}

by_value_arg_abi!(
    impl ArgAbi for bool { type Abi = u32; |js| js != 0 }
    with Option (is_none = js == U32_ABI_OPTION_SENTINEL)
);

impl OptionIntoWasmAbi for bool {
    #[inline]
    fn none() -> u32 {
        U32_ABI_OPTION_SENTINEL
    }
}

unsafe impl ErasableGeneric for bool {
    type Repr = bool;
}

impl Promising for bool {
    type Resolution = bool;
}

impl UpcastFrom<bool> for JsValue {}
impl UpcastFrom<bool> for JsOption<JsValue> {}
impl UpcastFrom<bool> for bool {}

impl IntoWasmAbi for char {
    type Abi = u32;

    #[inline]
    fn into_abi(self) -> u32 {
        self as u32
    }
}

by_value_arg_abi!(
    // SAFETY of the decode: checked in bindings.
    impl ArgAbi for char { type Abi = u32; |js| char::from_u32_unchecked(js) }
    with Option (is_none = js == U32_ABI_OPTION_SENTINEL)
);

impl OptionIntoWasmAbi for char {
    #[inline]
    fn none() -> u32 {
        U32_ABI_OPTION_SENTINEL
    }
}

unsafe impl ErasableGeneric for char {
    type Repr = char;
}

impl Promising for char {
    type Resolution = char;
}

impl UpcastFrom<char> for JsValue {}
impl UpcastFrom<char> for JsOption<JsValue> {}
impl UpcastFrom<char> for char {}

impl<T> IntoWasmAbi for *const T {
    type Abi = WasmWordRepr;

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self as usize as WasmWordRepr
    }
}

unsafe impl<T: ErasableGeneric> ErasableGeneric for *const T {
    type Repr = *const T::Repr;
}

impl<T, Target> UpcastFrom<*const T> for *const Target where Target: UpcastFrom<T> {}
impl<T, Target> UpcastFrom<*const T> for JsOption<*const Target> where Target: UpcastFrom<T> {}

impl<T> IntoWasmAbi for Option<*const T> {
    type Abi = f64;

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self.map(|ptr| ptr as usize as f64)
            .unwrap_or(F64_ABI_OPTION_SENTINEL)
    }
}

unsafe impl<T: ErasableGeneric> ErasableGeneric for Option<T> {
    type Repr = Option<<T as ErasableGeneric>::Repr>;
}

impl<T, Target> UpcastFrom<Option<T>> for Option<Target> where Target: UpcastFrom<T> {}
impl<T, Target> UpcastFrom<Option<T>> for JsOption<Option<Target>> where Target: UpcastFrom<T> {}

by_value_arg_abi!(
    impl<T> ArgAbi for *const T { type Abi = WasmWordRepr; |js| js as usize as *const T }
    with Option (type OptionAbi = f64; is_none = js == F64_ABI_OPTION_SENTINEL)
);

impl<T> IntoWasmAbi for *mut T {
    type Abi = WasmWordRepr;

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self as usize as WasmWordRepr
    }
}

impl<T> IntoWasmAbi for Option<*mut T> {
    type Abi = f64;

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self.map(|ptr| ptr as usize as f64)
            .unwrap_or(F64_ABI_OPTION_SENTINEL)
    }
}

by_value_arg_abi!(
    impl<T> ArgAbi for *mut T { type Abi = WasmWordRepr; |js| js as usize as *mut T }
    with Option (type OptionAbi = f64; is_none = js == F64_ABI_OPTION_SENTINEL)
);

impl<T> IntoWasmAbi for NonNull<T> {
    type Abi = WasmWordRepr;

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self.as_ptr() as usize as WasmWordRepr
    }
}

impl<T> OptionIntoWasmAbi for NonNull<T> {
    #[inline]
    fn none() -> Self::Abi {
        0 as WasmWordRepr
    }
}

by_value_arg_abi!(
    // SAFETY of the decode: checked in bindings.
    impl<T> ArgAbi for NonNull<T> { type Abi = WasmWordRepr; |js| NonNull::new_unchecked(js as usize as *mut T) }
    with Option (type OptionAbi = WasmWordRepr; |js| NonNull::new(js as usize as *mut T))
);

impl IntoWasmAbi for JsValue {
    type Abi = u32;

    #[inline]
    fn into_abi(self) -> u32 {
        let ret = self.idx;
        mem::forget(self);
        ret
    }
}

impl IntoWasmAbi for &JsValue {
    type Abi = u32;

    #[inline]
    fn into_abi(self) -> u32 {
        self.idx
    }
}

impl OptionIntoWasmAbi for JsValue {
    #[inline]
    fn none() -> u32 {
        crate::__rt::JSIDX_UNDEFINED
    }
}

impl OptionIntoWasmAbi for &JsValue {
    #[inline]
    fn none() -> u32 {
        crate::__rt::JSIDX_UNDEFINED
    }
}

by_value_arg_abi!(
    // The `None` peek borrows the heap value without freeing the index
    // (`ManuallyDrop`); on the `Some` path the decode takes ownership.
    impl ArgAbi for JsValue { type Abi = u32; |js| JsValue::_new(js) }
    with Option (is_none = ManuallyDrop::new(JsValue::_new(js)).is_undefined())
);

impl<T: OptionIntoWasmAbi> IntoWasmAbi for Option<T> {
    type Abi = T::Abi;

    #[inline]
    fn into_abi(self) -> T::Abi {
        match self {
            None => T::none(),
            Some(me) => me.into_abi(),
        }
    }
}

impl<T: OptionIntoWasmAbi + ErasableGeneric<Repr = JsValue> + Promising> Promising for Option<T> {
    type Resolution = Option<<T as Promising>::Resolution>;
}

impl<T: IntoWasmAbi> IntoWasmAbi for Clamped<T> {
    type Abi = T::Abi;

    #[inline]
    fn into_abi(self) -> Self::Abi {
        self.0.into_abi()
    }
}

impl<WbgS: Scope, T> ArgAbi<WbgS> for Clamped<T>
where
    T: ArgAbi<WbgS, Guard = Option<T>> + WasmDescribe,
{
    type Abi = <T as ArgAbi<WbgS>>::Abi;
    type Guard = Option<Clamped<T>>;
    type UnwindCheck = crate::__rt::marker::OwnedCheck<Clamped<T>>;

    #[inline(always)]
    unsafe fn arg_from_abi(js: Self::Abi) -> Self::Guard {
        Some(Clamped(<T as ArgAbi<WbgS>>::arg_from_abi(js).unwrap()))
    }

    #[cfg_attr(wasm_bindgen_unstable_test_coverage, coverage(off))]
    fn describe_arg() {
        <Clamped<T> as WasmDescribe>::describe();
    }
}

impl IntoWasmAbi for () {
    type Abi = ();

    #[inline]
    fn into_abi(self) {
        self
    }
}

by_value_arg_abi!(impl ArgAbi for () { type Abi = (); |js| js });

impl Promising for () {
    type Resolution = Undefined;
}

impl UpcastFrom<()> for JsValue {}
impl UpcastFrom<()> for () {}

unsafe impl ErasableGeneric for () {
    type Repr = ();
}

impl<T: WasmAbi<Prim3 = (), Prim4 = ()>> WasmAbi for Result<T, u32> {
    type Prim1 = T::Prim1;
    type Prim2 = T::Prim2;
    // The order of primitives here is such that we can pop() the possible error
    // first, deal with it and move on. Later primitives are popped off the
    // stack first.
    /// If this `Result` is an `Err`, the error value.
    type Prim3 = u32;
    /// Whether this `Result` is an `Err`.
    type Prim4 = u32;

    #[inline]
    fn split(self) -> (T::Prim1, T::Prim2, u32, u32) {
        match self {
            Ok(value) => {
                let (prim1, prim2, (), ()) = value.split();
                (prim1, prim2, 0, 0)
            }
            Err(err) => (Default::default(), Default::default(), err, 1),
        }
    }

    #[inline]
    fn join(prim1: T::Prim1, prim2: T::Prim2, err: u32, is_err: u32) -> Self {
        if is_err == 0 {
            Ok(T::join(prim1, prim2, (), ()))
        } else {
            Err(err)
        }
    }
}

impl<T, E> ReturnWasmAbi for Result<T, E>
where
    T: IntoWasmAbi,
    E: Into<JsValue>,
    T::Abi: WasmAbi<Prim3 = (), Prim4 = ()>,
{
    type Abi = Result<T::Abi, u32>;

    #[inline]
    fn return_abi(self) -> Self::Abi {
        match self {
            Ok(v) => Ok(v.into_abi()),
            Err(e) => {
                let jsval = e.into();
                Err(jsval.into_abi())
            }
        }
    }
}

unsafe impl<T: ErasableGeneric, E: ErasableGeneric> ErasableGeneric for Result<T, E> {
    type Repr = Result<<T as ErasableGeneric>::Repr, <E as ErasableGeneric>::Repr>;
}

impl<T: ErasableGeneric + Promising, E: ErasableGeneric> Promising for Result<T, E> {
    type Resolution = Result<<T as Promising>::Resolution, E>;
}

impl<T, E, TargetT, TargetE> UpcastFrom<Result<T, E>> for Result<TargetT, TargetE>
where
    TargetT: UpcastFrom<T>,
    TargetE: UpcastFrom<E>,
{
}
impl<T, E, TargetT, TargetE> UpcastFrom<Result<T, E>> for JsOption<Result<TargetT, TargetE>>
where
    TargetT: UpcastFrom<T>,
    TargetE: UpcastFrom<E>,
{
}

unsafe impl ErasableGeneric for JsError {
    type Repr = JsValue;
}

impl IntoWasmAbi for JsError {
    type Abi = <JsValue as IntoWasmAbi>::Abi;

    fn into_abi(self) -> Self::Abi {
        self.value.into_abi()
    }
}

by_value_arg_abi!(impl ArgAbi for JsError {
    type Abi = u32;
    |js| JsError {
        value: JsValue::_new(js),
    }
});

impl Promising for JsError {
    type Resolution = JsError;
}

impl UpcastFrom<JsError> for JsValue {}
impl UpcastFrom<JsError> for JsOption<JsValue> {}
impl UpcastFrom<JsError> for JsError {}

/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
// Note: this can't take `&[T]` because the `Into<JsValue>` impl needs
// ownership of `T`.
pub fn js_value_vector_into_abi<T: Into<JsValue>>(
    vector: Box<[T]>,
) -> <Box<[JsValue]> as IntoWasmAbi>::Abi {
    let js_vals: Box<[JsValue]> = vector.into_vec().into_iter().map(|x| x.into()).collect();

    js_vals.into_abi()
}

/// # ⚠️ Unstable
///
/// This is part of the internal [`convert`](crate::convert) module, **no
/// stability guarantees** are provided. Use at your own risk. See its
/// documentation for more details.
pub unsafe fn js_value_vector_from_abi<T: TryFromJsValue>(
    js: <JsValue as VectorFromWasmAbi>::Abi,
) -> Box<[T]> {
    let js_vals = Vec::from(<JsValue as VectorFromWasmAbi>::vector_from_abi(js));

    let mut result = Vec::with_capacity(js_vals.len());
    for value in js_vals {
        // We push elements one-by-one instead of using `collect` in order to improve
        // error messages. When using `collect`, this `expect_throw` is buried in a
        // giant chain of internal iterator functions, which results in the actual
        // function that takes this `Vec` falling off the end of the call stack.
        // So instead, make sure to call it directly within this function.
        //
        // This is only a problem in debug mode. Since this is the browser's error stack
        // we're talking about, it can only see functions that actually make it to the
        // final Wasm binary (i.e., not inlined functions). All of those internal
        // iterator functions get inlined in release mode, and so they don't show up.
        result.push(
            T::try_from_js_value(value).expect_throw("array contains a value of the wrong type"),
        );
    }
    result.into_boxed_slice()
}
