
use std::{path::PathBuf, time::Instant, f32::consts::PI};

use euclid::{UnknownUnit, point2};
use evdev::{Device};

use gumdrop::Options;


/// Monitor keyboard evdev devide. When a particular key is pressed, start monitoring touchpad/touchscreen
/// for specific gesture (clockwise or counterclockwise spins around a specific point), issuing commands
/// if enough spins are attained
/// Designed to integrate with [torchctl](https://github.com/vi/torchctl).
#[derive(Options)]
struct Opts {
    help: bool,
    #[options(short='k', default="/dev/input/event0")]
    keybd_file: PathBuf,
    #[options(short='t', default="/dev/input/event9")]
    touchpad_file: PathBuf,
    #[options(short='x', default="600")]
    center_x: i32,
    #[options(short='y', default="300")]
    center_y: i32,
    #[options(short='r', default="300")]
    radius: i32,
    #[options(short='F', default="2")]
    cw_spins_required: usize,
    #[options(short='R', default="2")]
    ccw_spins_required: usize,
    #[options(short='b', default="4000")]
    after_buttonpress_attention_time_ms: u32,
    #[options(short='a', default="4000")]
    after_spin_attention_time_ms: u32,
    #[options(short='Q', default="60000")]
    after_successful_cw_spin_sequence_attention_time: u32,
    #[options(short='K', default="57")]
    keycode_to_monitor: u16,
    /// Reset gesture attempt if this changes by more that this
    #[options(short='J', default="50")]
    max_jump_distance: u32,
}

enum State {
    WaitingForKeyboard,
    WaitingForTouches { deadline: Instant },
}

type Error = Box<dyn std::error::Error + Send + Sync>;


type Point = euclid::Point2D<f32, UnknownUnit>;
type Angle = euclid::Angle<f32>;

struct GestureState {

}

fn main() -> Result<(), Error> {
    let opts : Opts = gumdrop::parse_args_or_exit(gumdrop::ParsingStyle::AllOptions);
    let mut device = Device::open(opts.touchpad_file)?;

    let center : Point = point2(opts.center_x, opts.center_y).to_f32();
    let mut prev : Point = point2(0.0, 0.0);
    let mut prev_angle : Angle = Angle::zero();
    let sqradius = opts.radius as f32 * opts.radius as f32;
    let mut spinner = 0.0f32;
    loop {
        for ev in device.fetch_events()? {
            match ev.kind() {
                _ => (),
            }
        }
        if let Some(s) = device.cached_state().abs_vals() {
            let x = s[evdev::AbsoluteAxisType::ABS_MT_POSITION_X.0 as usize].value;
            let y = s[evdev::AbsoluteAxisType::ABS_MT_POSITION_Y.0 as usize].value;
            let p : Point = point2(x, y).to_f32();
            
            let v = p - center;
            if v.square_length() <= sqradius && v.square_length() > sqradius / 16.0 {
                let a = v.angle_from_x_axis();
                let d = prev_angle.angle_to(a);
                spinner += d.radians / PI / 2.0;
                println!("{:.1}", spinner);
                prev_angle = a;
            }

            prev = p;
        } 
    }
}
