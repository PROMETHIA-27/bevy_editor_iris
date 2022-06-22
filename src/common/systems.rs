use bevy::{ecs::schedule::ShouldRun, prelude::*};

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
