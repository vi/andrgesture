use std::{
    f32::consts::PI,
    os::unix::prelude::AsRawFd,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use euclid::{point2, UnknownUnit};
use evdev::Device;

use gumdrop::Options;
use nix::{
    fcntl::{FcntlArg, OFlag},
    poll::{PollFd, PollFlags},
};

/// Monitor keyboard evdev devide. When a particular key is pressed, start monitoring touchpad/touchscreen
/// for specific gesture (clockwise or counterclockwise spins around a specific point), issuing commands
/// if enough spins are attained
/// Designed to integrate with [torchctl](https://github.com/vi/torchctl).
#[derive(Options)]
struct Opts {
    help: bool,
    #[options(short = 'k', default = "/dev/input/event0")]
    keybd_file: PathBuf,
    #[options(short = 't', default = "/dev/input/event2")]
    touchpad_file: PathBuf,
    #[options(short = 'x', default = "1126")]
    center_x: i32,
    #[options(short = 'y', default = "748")]
    center_y: i32,
    #[options(short = 'r', default = "500")]
    radius: i32,
    #[options(short = 'F', default = "3")]
    cw_spins_required: usize,
    #[options(short = 'R', default = "2")]
    ccw_spins_required: usize,
    #[options(short = 'b', default = "4000")]
    after_buttonpress_attention_time_ms: u32,
    #[options(short = 'a', default = "4000")]
    after_spin_attention_time_ms: u32,
    #[options(short = 'Q', default = "60000")]
    after_successful_cw_spin_sequence_attention_time: u32,
    #[options(short = 'G', default = "300")]
    gesture_timeout_ms: u32,
    #[options(short = 'K', default = "116")]
    keycode_to_monitor: u16,
    /// Reset gesture attempt if this changes by more that this
    #[options(short = 'J', default = "200")]
    max_jump_distance: u32,
    #[options(short = 'D')]
    debug: bool,
    #[options(short='c', default = "/data/data/com.termux/files/home/bin/torchctl up")]
    cmdline_for_cw_spins: String,
    #[options(short='C', default = "/data/data/com.termux/files/home/bin/torchctl down")]
    cmdline_for_ccw_spins: String,
}

type Error = Box<dyn std::error::Error + Send + Sync>;

type Point = euclid::Point2D<f32, UnknownUnit>;
type Angle = euclid::Angle<f32>;

#[derive(derive_new::new)]
struct GestureState {
    deadline: Instant,
    prev: Point,
    prev_angle: Angle,
    #[new(default)]
    spinner: f32,
    #[new(default)]
    reacted_spin: f32,
}

enum State {
    WaitingForKeyboard,
    WaitingForTouches {
        deadline: Instant,
        gesture: Option<GestureState>,
    },
}

fn main() -> Result<(), Error> {
    let opts: Opts = gumdrop::parse_args_or_exit(gumdrop::ParsingStyle::AllOptions);
    let mut keydb = Device::open(opts.keybd_file)?;
    let mut touch = Device::open(opts.touchpad_file)?;

    nix::fcntl::fcntl(keydb.as_raw_fd(), FcntlArg::F_SETFL(OFlag::O_NONBLOCK))?;
    nix::fcntl::fcntl(touch.as_raw_fd(), FcntlArg::F_SETFL(OFlag::O_NONBLOCK))?;

    let center: Point = point2(opts.center_x, opts.center_y).to_f32();
    let sqradius = opts.radius as f32 * opts.radius as f32;
    let sqmaxd = opts.max_jump_distance as f32 * opts.max_jump_distance as f32;

    let mut state = State::WaitingForKeyboard;

    loop {
        let now = Instant::now();

        match &mut state {
            State::WaitingForKeyboard => {
                let mut polls = [PollFd::new(keydb.as_raw_fd(), PollFlags::POLLIN)];
                let stnow = SystemTime::now();
                nix::poll::poll(&mut polls, -1)?;

                for ev in keydb.fetch_events()? {
                    match ev.kind() {
                        evdev::InputEventKind::Key(k) => {
                            if opts.debug {
                                println!("Key {}", k.0);
                            }
                            if ev.value() == 1 && k.0 == opts.keycode_to_monitor {
                                let ts = ev.timestamp();
                                match ts.duration_since(stnow) {
                                    Ok(_) => {
                                        println!("Listening touchscreen");
                                        state = State::WaitingForTouches {
                                            deadline: Instant::now()
                                                + Duration::from_millis(
                                                    opts.after_buttonpress_attention_time_ms as u64,
                                                ),
                                            gesture: None,
                                        };
                                    }
                                    _ => {
                                        println!("Stale key event");
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            State::WaitingForTouches {
                deadline: touch_deadline,
                gesture,
            } => {
                let mut polls = [PollFd::new(touch.as_raw_fd(), PollFlags::POLLIN)];
                let n = nix::poll::poll(&mut polls, 20)?;

                if now > *touch_deadline {
                    println!("Stopping listening touchscreen");
                    state = State::WaitingForKeyboard;
                    continue;
                }

                if n == 0 {
                    continue;
                }
                for ev in touch.fetch_events()? {
                    match ev.kind() {
                        _ => (),
                    }
                }
                if let Some(s) = touch.cached_state().abs_vals() {
                    let x = s[evdev::AbsoluteAxisType::ABS_MT_POSITION_X.0 as usize].value;
                    let y = s[evdev::AbsoluteAxisType::ABS_MT_POSITION_Y.0 as usize].value;
                    if opts.debug {
                        println!("Touch {} {}", x, y);
                    }
                    let p: Point = point2(x, y).to_f32();

                    let v = p - center;
                    let inside_area =
                        v.square_length() <= sqradius && v.square_length() * 64.0 > sqradius;

                    if inside_area && gesture.is_none() {
                        let a = v.angle_from_x_axis();
                        *gesture = Some(GestureState::new(
                            now + Duration::from_millis(opts.gesture_timeout_ms as u64),
                            p,
                            a,
                        ));
                    }

                    let mut remove_gesture = false;
                    if let Some(ref mut g) = gesture {
                        if now > g.deadline {
                            remove_gesture = true;
                        }
                        if (p - g.prev).square_length() > sqmaxd {
                            remove_gesture = true;
                        }
                        if inside_area {
                            let a = v.angle_from_x_axis();
                            let d = g.prev_angle.angle_to(a);
                            g.deadline =
                                now + Duration::from_millis(opts.gesture_timeout_ms as u64);
                            g.spinner += d.radians / PI / 2.0;
                            if opts.debug {
                                println!("Spinner {:.1}", g.spinner);
                            }

                            let mut react_cw = false;
                            let mut react_ccw = false;
                            if g.reacted_spin > 0.5 {
                                if g.spinner >= g.reacted_spin + 1.0 {
                                    g.reacted_spin += 1.0;
                                    react_cw = true;
                                } else if g.spinner < g.reacted_spin - 1.0 {
                                    println!("Spinned in the opposite direction");
                                    remove_gesture = true;
                                }
                            } else if g.reacted_spin < -0.5 {
                                if g.spinner <= g.reacted_spin - 1.0 {
                                    g.reacted_spin -= 1.0;
                                    react_ccw = true;
                                } else if g.spinner > g.reacted_spin + 1.0 {
                                    println!("Spinned in the opposite direction");
                                    remove_gesture = true;
                                }
                            } else {
                                if g.spinner >= g.reacted_spin + 1.0 {
                                    g.reacted_spin += 1.0;
                                    react_cw = true;
                                } else if g.spinner < g.reacted_spin - 1.0 {
                                    g.reacted_spin -= 1.0;
                                    react_ccw = true;
                                }
                            }

                            if react_ccw || react_cw {
                                *touch_deadline = now + Duration::from_millis(opts.after_spin_attention_time_ms as u64);
                            }

                            let ctr : i32 = g.reacted_spin as i32;
                            let mut cmdline : Option<&str> = None;
                            if react_cw {
                                if ctr >= opts.cw_spins_required as i32 {
                                    *touch_deadline = now + Duration::from_millis(opts.after_successful_cw_spin_sequence_attention_time as u64);
                                    println!("SPIN CW {} !", ctr);
                                    cmdline = Some(opts.cmdline_for_cw_spins.as_ref());
                                } else {
                                    println!("SPIN CW {}", ctr);
                                }
                            }
                            if react_ccw {
                                if - ctr >= opts.ccw_spins_required as i32 {
                                    println!("SPIN CCW {} !", ctr);
                                    cmdline = Some(opts.cmdline_for_ccw_spins.as_ref());
                                } else {
                                    println!("SPIN CCW {}", ctr);
                                }
                            }

                            if let Some(cmd) = cmdline {
                                std::process::Command::new("sh").arg("-c").arg(cmd).spawn()?;
                            }

                            g.prev_angle = a;
                        }

                        g.prev = p;
                    }
                    if remove_gesture {
                        *gesture = None;
                    }
                } else {
                    if opts.debug {
                        println!("No absvals");
                    }
                }
            }
        }
    }
}
