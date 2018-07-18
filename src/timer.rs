//! Timing and measurement functions.
//!
//! ggez does not try to do any framerate limitation by default. If
//! you want to run at anything other than full-bore max speed all the
//! time, calling `thread::yield_now()` (or `timer::yield_now()` which
//! does the same thing) yield to the OS
//! so it has a chance to breathe before continuing with your game.
//! This should prevent it from using 100% CPU unless it really needs
//! to.  Enabling vsync by setting `vsync` in your `Conf` object is
//! generally the best way to cap your displayed framerate.
//!
//! For a more detailed tutorial in how to handle frame timings in games,
//! see <http://gafferongames.com/game-physics/fix-your-timestep/>

use crate::context::Context;

use std::cmp;
use std::f64;
use std::thread;
use std::time;

/// A simple buffer that fills
/// up to a limit and then holds the last
/// N items that have been inserted into it,
/// overwriting old ones in a round-robin fashion.
///
/// It's not quite a ring buffer 'cause you can't
/// remove items from it, it just holds the last N
/// things.
#[derive(Debug, Clone)]
struct LogBuffer<T>
where
    T: Clone,
{
    head: usize,
    size: usize,
    contents: Vec<T>,
}

impl<T> LogBuffer<T>
where
    T: Clone + Copy,
{
    fn new(size: usize, init_val: T) -> LogBuffer<T> {
        let mut v = Vec::with_capacity(size);
        v.resize(size, init_val);
        LogBuffer {
            head: 0,
            size,
            contents: v,
        }
    }

    /// Pushes a new item into the logbuffer, overwriting
    /// the oldest item in it.
    fn push(&mut self, item: T) {
        self.head = (self.head + 1) % self.contents.len();
        self.contents[self.head] = item;
        self.size = cmp::min(self.size + 1, self.contents.len());
    }

    /// Returns a slice pointing at the contents of the buffer.
    /// They are in *no particular order*, and if not all the
    /// slots are filled, the empty slots will be present but
    /// contain the initial value given to `new()`
    ///
    /// We're only using this to log FPS for a short time,
    /// so we don't care for the second or so when it's inaccurate.
    fn contents(&self) -> &[T] {
        &self.contents
    }

    /// Returns the most recent value in the buffer.
    fn latest(&self) -> T {
        self.contents[self.head]
    }
}

/// A structure that contains our time-tracking state.
#[derive(Debug)]
pub struct TimeContext {
    init_instant: time::Instant,
    last_instant: time::Instant,
    frame_durations: LogBuffer<time::Duration>,
    residual_update_dt: time::Duration,
    frame_count: usize,
}

// How many frames we log update times for.
const TIME_LOG_FRAMES: usize = 200;

impl TimeContext {
    /// Creates a new `TimeContext` and initializes the start to this instant.
    pub fn new() -> TimeContext {
        TimeContext {
            init_instant: time::Instant::now(),
            last_instant: time::Instant::now(),
            frame_durations: LogBuffer::new(TIME_LOG_FRAMES, time::Duration::new(0, 0)),
            residual_update_dt: time::Duration::from_secs(0),
            frame_count: 0,
        }
    }

    /// Update the state of the TimeContext to record that
    /// another frame has taken place.  Necessary for the FPS
    /// tracking and `check_update_time()` functions to work.
    ///
    /// It's usually not necessary to call this function yourself,
    /// `EventHandler::run()` will do it for you.
    pub fn tick(&mut self) {
        let now = time::Instant::now();
        let time_since_last = now - self.last_instant;
        self.frame_durations.push(time_since_last);
        self.last_instant = now;
        self.frame_count += 1;

        self.residual_update_dt += time_since_last;
    }
}

impl Default for TimeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the time between the start of the last frame and the current one;
/// in other words, the length of the last frame.
pub fn get_delta(ctx: &Context) -> time::Duration {
    let tc = &ctx.timer_context;
    tc.frame_durations.latest()
}

/// Gets the average time of a frame, averaged
/// over the last 200 frames.
pub fn get_average_delta(ctx: &Context) -> time::Duration {
    let tc = &ctx.timer_context;
    let init = time::Duration::new(0, 0);
    let sum = tc.frame_durations
        .contents()
        .iter()
        .fold(init, |d1, d2| d1 + *d2);
    sum / (tc.frame_durations.size as u32)
}

/// A convenience function to convert a Rust `Duration` type
/// to a (less precise but more useful) f64.
///
/// Does not make sure that the `Duration` is within the bounds
/// of the `f64`.
pub fn duration_to_f64(d: time::Duration) -> f64 {
    let seconds = d.as_secs() as f64;
    let nanos = f64::from(d.subsec_nanos());
    seconds + (nanos * 1e-9)
}

/// A convenience function to create a Rust `Duration` type
/// from a (less precise but more useful) f64.
///
/// Only handles positive numbers correctly.
pub fn f64_to_duration(t: f64) -> time::Duration {
    debug_assert!(t >= 0.0, "f64_to_duration passed a negative number!");
    let seconds = t.trunc();
    let nanos = t.fract() * 1e9;
    time::Duration::new(seconds as u64, nanos as u32)
}

/// Returns a `Duration` representing how long each
/// frame should be to match the given fps.
///
/// Approximately.
fn fps_as_duration(fps: u32) -> time::Duration {
    let target_dt_seconds = 1.0 / f64::from(fps);
    f64_to_duration(target_dt_seconds)
}

/// Gets the FPS of the game, averaged over the last
/// 200 frames.
pub fn get_fps(ctx: &Context) -> f64 {
    let duration_per_frame = get_average_delta(ctx);
    let seconds_per_frame = duration_to_f64(duration_per_frame);
    1.0 / seconds_per_frame
}

/// Returns the time since the game was initialized,
/// as reported by the system clock.
pub fn get_time_since_start(ctx: &Context) -> time::Duration {
    let tc = &ctx.timer_context;
    time::Instant::now() - tc.init_instant
}

/// This function will return true if the time since the
/// last `update()` call has been equal to or greater to
/// the update FPS indicated by the `target_fps`.
/// It keeps track of fractional frames, so if you want
/// 60 fps (16.67 ms/frame) and the game stutters so that
/// there is 40 ms between `update()` calls, this will return
/// `true` twice, and take the remaining 6.67 ms into account
/// in the next frame.
///
/// The intention is to for it to be called in a while loop
/// in your `update()` callback:
///
/// ```rust,ignore
/// fn update(&mut self, ctx: &mut Context) -> GameResult
///     while(timer::check_update_time(ctx, 60)) {
///         update_game_physics()?;
///     }
///     Ok(())
/// }
/// ```
pub fn check_update_time(ctx: &mut Context, target_fps: u32) -> bool {
    let timedata = &mut ctx.timer_context;

    let target_dt = fps_as_duration(target_fps);
    if timedata.residual_update_dt > target_dt {
        timedata.residual_update_dt -= target_dt;
        true
    } else {
        false
    }
}

/// Returns the fractional amount of a frame not consumed
/// by  `check_update_time()`.  For example, if the desired
/// update frame time is 40 ms (25 fps), and 45 ms have
/// passed since the last frame, `check_update_time()` will
/// return `true` and `get_remaining_update_time()` will
/// return 5 ms -- the amount of time "overflowing" from one
/// frame to the next.
///
/// The intention is for it to be called in your `draw()` callback
/// to interpolate phyisics states for smooth rendering.
/// (see <http://gafferongames.com/game-physics/fix-your-timestep/>)
pub fn get_remaining_update_time(ctx: &mut Context) -> time::Duration {
    ctx.timer_context.residual_update_dt
}

/// Pauses the current thread for the target duration.
/// Just calls `std::thread::sleep()` so it's as accurate
/// as that is (which is usually not very).
pub fn sleep(duration: time::Duration) {
    thread::sleep(duration);
}

/// Yields the current timeslice to the OS.
///
/// This just calls `std::thread::yield_now()` but it's
/// handy to have here.
pub fn yield_now() {
    thread::yield_now();
}

/// Gets the number of times the game has gone through its event loop.
///
/// Specifically, the number of times that `TimeContext::tick()` has been
/// called by it.
pub fn get_ticks(ctx: &Context) -> usize {
    ctx.timer_context.frame_count
}
