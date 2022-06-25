use std::sync::mpsc::{Receiver, Sender};

use bevy::ecs::schedule::ShouldRun;
use bevy::prelude::{Res, Time, World};
use futures_util::Future;

use crate::asynchronous::{self, RemoteThread, RemoteThreadError};
use crate::interface::StreamCounter;
use crate::{Interface, Message, StreamId};

pub fn run_on_timer(duration: f32) -> impl FnMut(Res<Time>) -> ShouldRun {
    struct Timer {
        duration: f32,
        elapsed: f32,
    }

    let mut timer = Timer {
        duration,
        elapsed: 0.0,
    };

    move |time: Res<Time>| {
        timer.elapsed += time.delta_seconds();

        if timer.elapsed > timer.duration {
            timer.elapsed = timer.elapsed - timer.duration;
            ShouldRun::Yes
        } else {
            ShouldRun::No
        }
    }
}

pub fn monitor_remote_thread<F: 'static + Future<Output = Result<(), RemoteThreadError>>>(
    run_fn: impl 'static
        + Fn(
            Receiver<(StreamId, Box<dyn Message>)>,
            Sender<(StreamId, Box<dyn Message>)>,
            StreamCounter,
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
