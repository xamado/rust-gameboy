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

use machine::Machine;
use joystick::JoystickButton;

const WINDOW_TITLE: &str = "rust-gameboy";
const BUFFER_WIDTH: u32 = 160;
const BUFFER_HEIGHT: u32 = 144;
const WINDOW_WIDTH: u32 = BUFFER_WIDTH * 4;
const WINDOW_HEIGHT: u32 = BUFFER_HEIGHT * 4;

// const BOX_SIZE: i16 = 64;

#[derive(Debug, Copy, Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cli_matches = App::new("rust-gameboy")
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
        .get_matches();

    let opt_rom_file = cli_matches.value_of("rom").unwrap();
    let opt_no_bootrom = cli_matches.occurrences_of("no-bootrom") > 0;

    let sdl = SDL::init(InitFlags::default())?;
    let mut window = sdl.create_raw_window(WINDOW_TITLE, WindowPosition::Centered, WINDOW_WIDTH, WINDOW_HEIGHT, 0)?;
    
    let surface_texture = SurfaceTexture::new(BUFFER_WIDTH, BUFFER_HEIGHT, &window);
    let mut pixels = PixelsBuilder::new(BUFFER_WIDTH, BUFFER_HEIGHT, surface_texture)
         .request_adapter_options(wgpu::RequestAdapterOptions {
             power_preference: wgpu::PowerPreference::HighPerformance,
             compatible_surface: None,
         })
         .enable_vsync(true)
         .build()?;
     
    pixels.resize(WINDOW_WIDTH, WINDOW_HEIGHT);

    let mut machine = Machine::new();
    machine.start(opt_no_bootrom);
    machine.load_rom(opt_rom_file);

    let mut instant = Instant::now();
    let frame_time: f32 = 1.0 / 60.0;

    'game_loop: loop {
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

            // Resize the window
            Some(Event::Window(WindowEvent {
                event: WindowEventEnum::Resized { w, h },
                ..
            })) => pixels.resize(w as u32, h as u32),

            _ => (),
        }

        machine.step();

        let mut screen = machine.get_screen().borrow_mut();
        if screen.is_vblank() {
            let fb = screen.get_framebuffer();

            let colors: [Color; 4] = [
                Color { r: 255, g: 255, b: 255 },
                Color { r: 126, g: 126, b: 126 },
                Color { r: 63, g: 63, b: 63 },
                Color { r: 0, g: 0, b: 0 }
            ];
    
            let frame = pixels.get_frame();
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let fb_idx = fb[i] as usize;
                let c = colors[fb_idx];
                pixel[0] = c.r;
                pixel[1] = c.g;
                pixel[2] = c.b;
                pixel[3] = 255;
            }

            screen.set_vblank(false);
            
            // Draw the current frame
            pixels.render()?;

            // Sync to 60hz
            let elapsed = instant.elapsed().as_secs_f32();
            if elapsed < frame_time {
                sleep(Duration::from_secs_f32(frame_time - elapsed));
            }

            instant = Instant::now();

            // Update window title
            let window_title = format!("{} ({}ms)", WINDOW_TITLE, (elapsed * 1000.0) as u32);
            window.set_title(&window_title);            
        }

    }

    Ok(())
}

