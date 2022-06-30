// TODO: Probably split this off into its own crate
// TODO: Reference versions

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
        let input: std::boxed::Box<dyn std::any::Any> = $input;
        let output = typematch!(@first_unroll input {} $($body)*);
        $crate::macros::bruh();
        output
    }};

    (
        @first_unroll
        $input:ident
        {}
        default => $body:expr
    ) => {
        $body
    };

    (
        @first_unroll
        $input:ident
        {}
        $name:ident : $ty:ty => $body:expr,
        default => $default_body:expr,
    ) => {
        if <dyn std::any::Any>::is::<$ty>(&*$input) {
            let $name = std::boxed::Box::<dyn std::any::Any>::downcast::<$ty>($input).unwrap();
            $body
        }
        else {
            $default_body
        }
    };

    (
        @first_unroll
        $input:ident
        {}
        $name:ident : $ty:ty => $body:expr,
        $($rest:tt)+
    ) => {
        typematch!(
            @unroll
            $input
            {
                if <dyn std::any::Any>::is::<$ty>(&*$input) {
                    let $name = std::boxed::Box::<dyn std::any::Any>::downcast::<$ty>($input).unwrap();
                    $body
                }
            }
            $($rest)*
        )
    };

    (
        @unroll
        $input:ident
        { $($stored:tt)+ }
        $name:ident : $ty:ty => $body:expr,
        default => $default_body:expr,
    ) => {
        $($stored)+
        else if <dyn std::any::Any>::is::<$ty>(&*$input) {
            let $name = std::boxed::Box::<dyn std::any::Any>::downcast::<$ty>($input).unwrap();
            $body
        }
        else {
            $default_body
        }
    };

    (
        @unroll
        $input:ident
        { $($stored:tt)+ }
        $name:ident : $ty:ty => $body:expr,
        $($rest:tt)+
    ) => {
        typematch!(
            @unroll
            $input
            {
                $($stored)+
                else if <dyn std::any::Any>::is::<$ty>(&*$input) {
                    let $name = std::boxed::Box::<dyn std::any::Any>::downcast::<$ty>($input).unwrap();
                    $body
                }
            }
            $($rest)*
        )
    };

    (
        @first_unroll
        $input:ident
        {}
        $name:ident : $ty:ty => $body:expr,
    ) => {
        typematch!(@no_default_err)
    };

    (
        @unroll
        $input:ident
        { $($stored:tt)+ }
        $name:ident : $ty:ty => $body:expr,
    ) => {
        typematch!(@no_default_err)
    };

    (
        @no_default_err
    ) => {
        compile_error!(r#"must include a default case to match all other types.

Example:
typematch!(value, {
    int: i32 => int,
    uint: u32 => uint as i32,
    default => panic!("Not a 32-bit integer!"), 
});"#)
    };
}

#[test]
fn typematch() {
    use std::any::Any;

    use crate::message::messages::{CloseTransaction, Ping};

    let message = Ping;
    let message = Box::new(message) as Box<dyn Any>;

    let value = typematch!(message, {
        close: CloseTransaction => 24,
        ping: Ping => 12,
        default => panic!(),
    });

    assert_eq!(value, 12);
}

// TODO: With Generic Associated Types, I can make typematch generic over references as well
// pub trait Typematch {
//     type Target<T>;

//     fn is<T: Any>(&self) -> bool;

//     fn downcast<T: Any>(self) -> Self::Target<T>;
// }
