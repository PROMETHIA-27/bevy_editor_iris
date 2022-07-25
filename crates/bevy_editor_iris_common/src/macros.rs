// TODO: Probably split this off into its own crate
// TODO: Reference versions

/// Match on the type of a `Box<dyn Any>`.
///
/// There must be a default case to handle all non-matched types.
///
/// ## Example:
/// ```
/// # use std::any::Any;
/// # use bevy_editor_iris_common::typematch;
/// let i = 12;
/// let any = Box::new(i) as Box<dyn Any>;
///
/// let value: usize = typematch!(any, {
///     int: i32 => int as usize,
///     uint: u32 => 0,
///     default => panic!("not a 32-bit integer!")
/// });
///
/// assert_eq!(value, 12);
/// ```
#[macro_export]
macro_rules! typematch {
    ($input:expr, { $($body:tt)* }) => {{
        let input: ::std::boxed::Box<dyn ::std::any::Any> = $input;
        let output = $crate::typematch!(@first_unroll input {} $($body)*);
        #[allow(unreachable_code)]
        output
    }};

    (
        @first_unroll
        $input:ident
        {}
        default => $body:expr $(,)?
    ) => {
        $body
    };

    (
        @first_unroll
        $input:ident
        {}
        $name:ident : $ty:ty => $body:expr,
        default => $default_body:expr $(,)?
    ) => {
        if <dyn ::std::any::Any>::is::<$ty>(&*$input) {
            let $name = *::std::boxed::Box::<dyn ::std::any::Any>::downcast::<$ty>($input).unwrap();
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
        $crate::typematch!(
            @unroll
            $input
            {
                if <dyn ::std::any::Any>::is::<$ty>(&*$input) {
                    let $name = *::std::boxed::Box::<dyn ::std::any::Any>::downcast::<$ty>($input).unwrap();
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
        default => $default_body:expr $(,)?
    ) => {
        $($stored)+
        else if <dyn ::std::any::Any>::is::<$ty>(&*$input) {
            let $name = *::std::boxed::Box::<dyn ::std::any::Any>::downcast::<$ty>($input).unwrap();
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
        $crate::typematch!(
            @unroll
            $input
            {
                $($stored)+
                else if <dyn ::std::any::Any>::is::<$ty>(&*$input) {
                    let $name = *::std::boxed::Box::<dyn ::std::any::Any>::downcast::<$ty>($input).unwrap();
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
        $crate::typematch!(@no_default_err)
    };

    (
        @unroll
        $input:ident
        { $($stored:tt)+ }
        $name:ident : $ty:ty => $body:expr,
    ) => {
        $crate::typematch!(@no_default_err)
    };

    (
        @no_default_err
    ) => {
        ::std::compile_error!(r#"must include a default case to match all other types.

Example:
typematch!(value, {
    int: i32 => int,
    uint: u32 => uint as i32,
    default => panic!("Not a 32-bit integer!"), 
});"#)
    };
}

// TODO: With Generic Associated Types, I can make typematch generic over references as well
// pub trait Typematch {
//     type Target<T>;

//     fn is<T: Any>(&self) -> bool;

//     fn downcast<T: Any>(self) -> Self::Target<T>;
// }

// TODO: Tbh this should be a proc macro but ¯\_(ツ)_/¯ I'll make it a standalone crate later
/// A state machine macro to make a state machine more ergonomic to write. The state is represented as an enum.
/// Equivalent to `loop { state = match state { ... }}`, but easier to read.
///
/// ## Examples
/// ```
/// # use bevy_editor_iris_common::state_machine;
/// let two: u32 = state_machine!(
///     state {
///         A,
///         B(i32),
///         C {
///             string: String
///         },
///     }
///     run A => {
///         A => B(2),
///         B(num) => {
///             C { string: num.to_string() }
///         }
///         C { string } => {
///             break string.parse::<u32>().unwrap();
///         },
///     }
/// );
///
/// assert_eq!(two, 2);
/// ```
///
/// You can also use an extern enum as state with this form:
/// ```
/// # use bevy_editor_iris_common::state_machine;
/// enum State {
///     Input(i32),
///     Multiply(u32),
///     Output(usize),
/// }
///
/// let five = state_machine!(
///     extern state = State,
///     run Input(1) => {
///         Input(input) => Multiply((input + 1) as u32),
///         Multiply(mul) => Output((mul * 2) as usize),
///         Output(out) => break out + 1,
///     }
/// );
///
/// # assert_eq!(five, 5);
/// ```
///
/// With extern states you can even pass in or out a state value:
/// ```
/// # use bevy_editor_iris_common::state_machine;
/// #[derive(Debug, PartialEq)]
/// enum TrafficLight {
///     Red,
///     Yellow,
///     Green,
/// }
/// #[derive(Debug, PartialEq)]
/// enum Action {
///     Starting,
///     Stopping,
/// }
///
/// let start = TrafficLight::Red;
/// let action = Action::Starting;
///
/// let (next, action) = state_machine!(
///     extern state = TrafficLight,
///     run start => {
///         Red => match action {
///             Action::Starting => break (Yellow, Action::Starting),
///             Action::Stopping => break (Red, Action::Starting),
///         },
///         Yellow => match action {
///             Action::Starting => break (Green, Action::Starting),
///             Action::Stopping => break (Red, Action::Stopping),
///         },
///         Green => match action {
///             Action::Starting => break (Green, Action::Stopping),
///             Action::Stopping => break (Yellow, Action::Stopping),
///         }
///     }
/// );
///
/// assert_eq!((next, action), (TrafficLight::Yellow, Action::Starting));
/// ```
#[macro_export]
macro_rules! state_machine {
    ( // Entrypoint
        state { $($states:tt)* }
        run $start_value:expr => {
            $($state:pat => $body:expr $(,)?)+
        }
    ) => {
        {
            enum LocalMachineState {
                $($states)*
            }

            let mut state: LocalMachineState = {
                use LocalMachineState::*;
                $start_value
            };

            $crate::state_machine!(
                @behaviors
                LocalMachineState,
                state
                { $($state => $body),+ }
            )
        }
    };

    ( // External state entrypoint
        extern state = $state_enum:path,
        run $start_value:expr => {
            $($state:pat => $body:expr $(,)?)+
        }
    ) => {
        {
            let mut state: $state_enum = {
                use $state_enum::*;
                $start_value
            };

            $crate::state_machine!(
                @behaviors
                $state_enum,
                state
                { $($state => $body),+ }
            )
        }
    };

    (
        @behaviors
        $state_enum:path,
        $state_value:ident
        { $($state:pat => $body:expr),+ }
    ) => {
        #[allow(unused_labels)]
        'machine: loop {
            use $state_enum::*;
            $state_value = match $state_value {
                $($state => $body),+
            }
        }
    };
}
