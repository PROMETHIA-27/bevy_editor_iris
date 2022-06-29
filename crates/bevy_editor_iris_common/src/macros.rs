// TODO: Probably split this off into its own crate

/// Match on the type of a `dyn Any` or another dyn trait such that
/// `Trait: Any`.
///
/// ## Example:
/// ```
/// # use std::any::Any;
///
/// let i = 12;
/// let any = Box::new(i) as Box<dyn Any>;
///
/// let value: usize = typematch (i, {
///     int: i32 => int as usize,
///     uint: u32 => todo!(),
/// });
///
/// assert_eq!(value, 12);
/// ```
#[macro_export]
macro_rules! typematch {
    ($input:expr, { $($body:tt)* }) => {{
        let input = $input;
        typematch!(@unroll input $($body)*)
    }};

    (
        @unroll
        $input:ident
        $name:ident : $ty:ty => $body:expr,
        $($rest:tt)*
    ) => {
        if std::any::Any::type_id(&$input) == std::any::TypeId::of::<$ty>() {
            let $name = $input.downcast::<$ty>().unwrap();
            $body
        }
    };
}

#[test]
fn typematch() {
    let message = crate::message::messages::Ping;
    let message = Box::new(message) as Box<dyn crate::message::Message>;
    let borrow = &*message;
    borrow.downcast();

    let value = typematch!(borrow, {
        ping: Ping => 12,
    });
}
