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
        let $var = $init;
    };
}

#[macro_export]
macro_rules! rooted_vec {
    (let mut $name:ident) => {
        let mut $name = Vec::new();
    };
    (let $name:ident <- $iter:expr) => {
        let $name: Vec<_> = $iter.collect();
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
