use std::time::Duration;

use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::{Res, Time, World};
use futures_lite::Future;

use crate::asynchronous::{self, OpeningReceiver, OpeningSender, RemoteThread};
use crate::error::RemoteThreadError;
use crate::interface::Interface;

/// Creates a run criteria for running a system on an interval of `duration`.
///
/// ## Example:
/// ```
/// # use std::time::Duration;
/// # use bevy::prelude::*;
/// #
/// # fn my_system() {}
/// #
/// App::new()
///     .add_system_set(
///         SystemSet::new()
///             .with_run_criteria(run_on_timer(Duration::from_secs(1)))
///             .with_system(my_system)
///     );
/// ```
pub fn run_on_timer(duration: Duration) -> impl FnMut(Res<Time>) -> ShouldRun {
    struct Timer {
        duration: Duration,
        elapsed: Duration,
    }

    let mut timer = Timer {
        duration,
        elapsed: Duration::ZERO,
    };

    move |time: Res<Time>| {
        timer.elapsed += time.delta();

        if timer.elapsed > timer.duration {
            timer.elapsed = timer.elapsed - timer.duration;
            ShouldRun::Yes
        } else {
            ShouldRun::No
        }
    }
}

/// Monitors the remote thread until it closes; when it does, uses the given run function to
/// reopen it if the closure was unexpected.
pub fn monitor_remote_thread<F: 'static + Future<Output = Result<(), RemoteThreadError>>>(
    run_fn: impl 'static
        + Fn(
            OpeningSender,
            OpeningReceiver,
            // StreamCounter,
        ) -> F
        + Send
        + Sync
        + Copy,
) -> impl 'static + Fn(&mut World) {
    move |world| {
        let RemoteThread(thread) = world
            .remove_resource::<RemoteThread>()
            .expect("RemoteThread resource unexpectedly removed");

        if !thread.is_finished() {
            world.insert_resource(RemoteThread(thread));
        } else {
            match thread.join() {
                Ok(Ok(())) => {
                    eprintln!("Remote thread closed normally. Not reopening.");
                    return;
                }
                Ok(Err(err)) => eprintln!("Remote thread closed with error {err:?}! Reopening."),
                Err(_) => eprintln!("Remote thread closed with an unknown error! Reopening."),
            }

            _ = world.remove_resource::<Interface>();
            asynchronous::open_remote_thread(run_fn)(world);
        }
    }
}
