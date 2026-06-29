// Macro stubs from mozjs — placed at crate root via #[macro_use]
// These are used by generated WebIDL bindings and script_bindings source.

#[macro_export]
macro_rules! new_jsjitinfo_bitfield_1 {
    (
        $type_: expr,
        $aliasSet_: expr,
        $returnType_: expr,
        $isInfallible: expr,
        $isMovable: expr,
        $isEliminatable: expr,
        $isAlwaysInSlot: expr,
        $isLazilyCachedInSlot: expr,
        $isTypedMethod: expr,
        $slotIndex: expr,
    ) => {
        0 | (($type_ as u32) << 0u32)
            | (($aliasSet_ as u32) << 4u32)
            | (($returnType_ as u32) << 8u32)
            | (($isInfallible as u32) << 16u32)
            | (($isMovable as u32) << 17u32)
            | (($isEliminatable as u32) << 18u32)
            | (($isAlwaysInSlot as u32) << 19u32)
            | (($isLazilyCachedInSlot as u32) << 20u32)
            | (($isTypedMethod as u32) << 21u32)
            | (($slotIndex as u32) << 22u32)
    };
}

#[macro_export]
macro_rules! rooted {
    (&in($cx:expr) let $($var:ident),+ = $init:expr) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let __init = $init;
        let $($var),+ = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, __init) };
    };
    (&in($cx:expr) let mut $var:ident = $init:expr) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let __init = $init;
        let mut $var = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, __init) };
    };
    (&in($cx:expr) let mut $var:ident: $ty:ty = $init:expr) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let __init = $init;
        let mut $var: $crate::gc::RootedGuard<'_, $ty> = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, __init) };
    };
    (&in($cx:expr) let mut $var:ident: $ty:ty) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let mut $var: $crate::gc::RootedGuard<'_, $ty> = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, ::std::mem::zeroed()) };
    };
    (in($cx:expr) let $($var:ident),+ = $init:expr) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let __init = $init;
        let $($var),+ = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, __init) };
    };
    (in($cx:expr) let mut $var:ident = $init:expr) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let __init = $init;
        let mut $var = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, __init) };
    };
    (in($cx:expr) let mut $var:ident: $ty:ty = $init:expr) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let __init = $init;
        let mut $var: $crate::gc::RootedGuard<'_, $ty> = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, __init) };
    };
    (in($cx:expr) let mut $var:ident: $ty:ty) => {
        let mut __root = ::std::mem::MaybeUninit::uninit();
        let mut $var: $crate::gc::RootedGuard<'_, $ty> = unsafe { $crate::gc::RootedGuard::new(&$cx, &mut __root, ::std::mem::zeroed()) };
    };
}

#[macro_export]
macro_rules! auto_root {
    (&in($cx:expr) let $var:ident = $init:expr) => {
        let $var = $crate::rust::CustomAutoRooterGuard::from($init);
    };
}

#[macro_export]
macro_rules! rooted_vec {
    (let mut $name:ident) => {
        let mut __rootable_vec = $crate::gc::RootableVec::new_unrooted();
        let mut $name = $crate::gc::RootedVec::new(&mut __rootable_vec);
    };
    (let mut $name:ident <- $iter:expr) => {
        let mut __rootable_vec = $crate::gc::RootableVec::new_unrooted();
        let mut $name = $crate::gc::RootedVec::from_iter(&mut __rootable_vec, $iter);
    };
    (let $name:ident <- $iter:expr) => {
        let mut __rootable_vec = $crate::gc::RootableVec::new_unrooted();
        let $name = $crate::gc::RootedVec::from_iter(&mut __rootable_vec, $iter);
    };
}

#[macro_export]
macro_rules! typedarray {
    (&in($cx:expr) $($t:tt)*) => {
        typedarray!(in(unsafe { $cx.raw_cx() }) $($t)*);
    };
    (in($cx:expr) let $name:ident : $ty:ident = $init:expr) => {
        let mut __array =
            $crate::typedarray::$ty::from($init).map($crate::rust::CustomAutoRooter::new);
        let $name = match __array.as_mut() {
            Some(rooter) => Some(rooter.root($cx)),
            None => None,
        };
    };
    (in($cx:expr) let mut $name:ident : $ty:ident = $init:expr) => {
        let mut __array =
            $crate::typedarray::$ty::from($init).map($crate::rust::CustomAutoRooter::new);
        let mut $name = match __array.as_mut() {
            Some(rooter) => Some(rooter.root($cx)),
            None => None,
        };
    };
}

#[macro_export]
macro_rules! glue_stub {
    (pub fn $name:ident($($arg:ident : $ty:ty),*) -> $ret:ty) => {
        pub fn $name($($arg: $ty),*) -> $ret {
            Default::default()
        }
    };
    (pub fn $name:ident($($arg:ident : $ty:ty),*)) => {
        pub fn $name($($arg: $ty),*) {}
    };
}
