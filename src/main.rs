#![deny(clippy::all)]
#![forbid(unsafe_code)]

use beryllium::*;
use pixels::{PixelsBuilder, SurfaceTexture, wgpu};
use std::time::{Duration, Instant};
use std::thread::sleep;
use clap::{Arg, App};

mod cpu;
mod rom;
mod memory;
mod machine;
mod ppu;
mod memorybus;
mod screen;
mod joystick;
mod iomapped;
mod timer;
mod bitutils;
mod serial;
mod debugger;
mod bootrom;
mod apu;

use machine::Machine;
use joystick::JoystickButton;
use debugger::Debugger;
use rom::ROM;

const WINDOW_TITLE: &str = "rust-gameboy";
const BUFFER_WIDTH: u32 = 160;
const BUFFER_HEIGHT: u32 = 144;
const WINDOW_WIDTH: u32 = BUFFER_WIDTH * 4;
const WINDOW_HEIGHT: u32 = BUFFER_HEIGHT * 4;

#[derive(Debug, Copy, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse CLI args
    let cli_matches = get_cli_matches();
    let opt_rom_file = cli_matches.value_of("rom").unwrap();
    let opt_no_bootrom = cli_matches.occurrences_of("no-bootrom") > 0;
    let opt_breakpoints = cli_matches.value_of("breakpoints").unwrap_or("");
    let opt_watchpoints = cli_matches.value_of("watchpoints").unwrap_or("");
    
    let sdl = SDL::init(InitFlags::default())?;
    let mut window = sdl.create_raw_window(WINDOW_TITLE, WindowPosition::Centered, WINDOW_WIDTH, WINDOW_HEIGHT, 0)?;
    
    let surface_texture = SurfaceTexture::new(BUFFER_WIDTH, BUFFER_HEIGHT, &window);
    let mut pixels = PixelsBuilder::new(BUFFER_WIDTH, BUFFER_HEIGHT, surface_texture)
         .request_adapter_options(wgpu::RequestAdapterOptions {
             power_preference: wgpu::PowerPreference::HighPerformance,
             compatible_surface: None,
         })
         .enable_vsync(false)
         .build()?;
     
    pixels.resize(WINDOW_WIDTH, WINDOW_HEIGHT);

    let request = AudioQueueRequest {
        frequency: 44100,
        sample_format: AudioFormat::I16_SYS,
        sample_count: 4096,
        channels: AudioChannels::Stereo,
        allow_frequency_change: false,
        allow_format_change: false,
        allow_channels_change: false
    };

    let device_name = sdl.get_audio_playback_device_name(0).expect("No audio device");
    let queue: AudioQueue = sdl.open_audio_queue(Some(&device_name[..]), request)?;
    queue.set_paused(false);
    println!("device name: {}", device_name);

    let mut debugger = Debugger::new();
    
    // Add breakpoints
    if !opt_breakpoints.is_empty() {
        let breakpoints = opt_breakpoints.split(',');
        for bp in breakpoints {
            let addr = u16::from_str_radix(&bp[2..6], 16)?;
            debugger.add_breakpoint(addr);
        }
    }

    // Add watchpoints
    if !opt_watchpoints.is_empty() {
        let watchpoints = opt_watchpoints.split(',');
        for wp in watchpoints {
            let addr = u16::from_str_radix(&wp[2..6], 16)?;
            debugger.add_watchpoint(addr);
        }
    }
    
    let mut rom = ROM::new();
    rom.open(opt_rom_file);

    let mut machine = Machine::new(rom);
    machine.start(opt_no_bootrom);
    machine.attach_debugger(debugger);

    let mut instant = Instant::now();
    let frame_time: f32 = 1.0 / 60.0;

    'game_loop: loop {
        // process input
        match sdl.poll_events().and_then(Result::ok) {
            // Close events
            Some(Event::Quit { .. }) => break 'game_loop,
            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                ..
            })) if key == Keycode::ESCAPE => break 'game_loop,

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::A => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::Start, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::S => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::Select, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::Z => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::A, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::X => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::B, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::LEFT => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::Left, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::RIGHT => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::Right, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::UP => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::Up, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::DOWN => {
                machine.get_joystick().borrow_mut().inject(JoystickButton::Down, value);
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::F10 && value => {
                machine.debugger_step();
            }

            Some(Event::Keyboard(KeyboardEvent {
                key: KeyInfo { keycode: key, .. },
                is_pressed: value,
                ..
            })) if key == Keycode::F5 && value => {
                machine.debugger_continue();
            }

            // Resize the window
            Some(Event::Window(WindowEvent {
                event: WindowEventEnum::Resized { w, h },
                ..
            })) => pixels.resize(w as u32, h as u32),

            _ => (),
        }

        // process logic
        'emulator_loop: loop {
            machine.step();

            let screen = machine.get_screen().borrow();
            if screen.is_vblank() {
                break 'emulator_loop;
            }    

            if machine.is_stopped() {
                break 'emulator_loop;
            }
        }

        let mut screen = machine.get_screen().borrow_mut();
        if screen.is_vblank() {
            // Queue audio samples first
            let audio_buffer = machine.get_audio_buffer();
            let len = audio_buffer.len();
            let s = bytemuck::cast_slice(&audio_buffer[0..len]);

            if let Err(e) = queue.queue_audio(&s) {
                println!("Error queing audio: {:?}", e);
            }

            // Update pixels' framebuffer
            let fb = screen.get_framebuffer();
            let frame = pixels.get_frame();
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let c = fb[i];
                pixel[0] = (c & 0xFF) as u8;
                pixel[1] = ((c >> 8) & 0xFF) as u8;
                pixel[2] = ((c >> 16) & 0xFF) as u8;
                pixel[3] = 255;
            }

            // Draw the current frame
            pixels.render()?;
            screen.set_vblank(false);

            // Sync to 60hz, only if we have enough audio samples
            let elapsed = instant.elapsed().as_secs_f32();
            if queue.get_queued_byte_count() > 8192 && elapsed < frame_time {
                sleep(Duration::from_secs_f32(frame_time - elapsed));
            }
            else {
                // println!("skip");
            }

            // Update window title
            let window_title = format!("{} ({}ms)", WINDOW_TITLE, (elapsed * 1000.0) as u32);
            window.set_title(&window_title);

            instant = Instant::now();
        }
    }

    // Stop the machine
    machine.stop();

    Ok(())
}

fn get_cli_matches() -> clap::ArgMatches<'static> {
    App::new("rust-gameboy")
        .version("0.1")
        .author("Xavier Amado <xamado@gmail.com")
        .about("GB emulator written in Rust")
        .arg(Arg::with_name("rom")
            .long("rom")
            .help("Specify rom to load")
            .required(true)
            .takes_value(true)
        )
        .arg(Arg::with_name("no-bootrom")
            .long("no-bootrom")
            .help("Avoid the bootrom and just start ROM directly")
            .takes_value(false)
        )
        .arg(Arg::with_name("breakpoints")
            .long("breakpoints")
            .short("bp")
            .help("Comma separated list of breakpoint addresses")
            .takes_value(true)
        )
        .arg(Arg::with_name("watchpoints")
            .long("watchpoints")
            .short("wp")
            .help("Comma separated list of memory addresses to watch")
            .takes_value(true)
        )
        .get_matches()
}