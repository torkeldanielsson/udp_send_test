#![windows_subsystem = "windows"]

use anyhow::{anyhow, Result};
use glium::{Display, Surface};
use imgui::{im_str, ColorEdit, Condition, Context, FontSource, StyleColor, Ui, Window};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::env;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};
use winit::dpi::{LogicalPosition, LogicalSize};

#[derive(Debug)]
struct State {
    mouse_state: MouseState,
    last_mouse_state: MouseState,

    pan: [f64; 2],
    panning: bool,
    auto_pan: bool,
    scroll_factor: f64,

    quit: bool,
    fullscreen: bool,

    port: u16,
    address: String,
}

impl State {
    fn new(port: u16, address: String) -> State {
        State {
            mouse_state: MouseState::new(),
            last_mouse_state: MouseState::new(),
            pan: [0.0, 0.0],
            panning: false,
            auto_pan: true,
            scroll_factor: 2.2,
            quit: false,
            fullscreen: false,
            port: port,
            address: address,
        }
    }
}

fn draw_gui(ui: &Ui, state: &mut State) {
    let view_size = ui.io().display_size;
    let mut scale = f64::exp(state.scroll_factor as f64);

    Window::new(im_str!("Main"))
        .size([view_size[0] - 400.0, view_size[1]], Condition::Always)
        .position([400.0, 0.0], Condition::Always)
        .movable(false)
        .resizable(false)
        .title_bar(false)
        .collapsible(false)
        .menu_bar(false)
        .focused(false)
        .build(ui, || {
            if !ui.is_item_hovered() && ui.is_window_hovered() && state.mouse_state.pressed.0 {
                state.panning = true;
            }

            if !state.mouse_state.pressed.0 {
                state.panning = false;
            }

            if (!ui.is_item_hovered() && ui.is_window_hovered()) || state.panning {
                if state.mouse_state.pressed.0 {
                    let dx = state.last_mouse_state.pos.0 - state.mouse_state.pos.0;
                    let dy = state.last_mouse_state.pos.1 - state.mouse_state.pos.1;

                    state.pan[0] += dx;
                    state.pan[1] += dy;

                    if (dx, dy) != (0.0, 0.0) {
                        state.auto_pan = false;
                    }
                }

                if state.mouse_state.wheel != (0.0, 0.0) {
                    state.auto_pan = false;

                    if state.mouse_state.wheel.1 != 0.0 {
                        let mouse_centered_x = state.mouse_state.pos.0 as f64;

                        let new_scroll_factor = (state.scroll_factor
                            - state.mouse_state.wheel.1 as f64 / 10.0)
                            .max(-5.5)
                            .min(5.5);

                        let last_scale = f64::exp(state.scroll_factor);
                        let new_scale = f64::exp(new_scroll_factor);

                        let mouse_centered_last_scale_x =
                            (state.pan[0] - 400.0 + mouse_centered_x) / last_scale;
                        let mouse_centered_scale_x =
                            (state.pan[0] - 400.0 + mouse_centered_x) / new_scale;

                        state.pan[0] -=
                            (mouse_centered_last_scale_x - mouse_centered_scale_x) * last_scale;

                        state.scroll_factor = new_scroll_factor;
                        scale = f64::exp(state.scroll_factor as f64);
                    }

                    state.pan[0] += state.mouse_state.wheel.0 as f64;
                }
            }
        });
}

#[derive(Debug, Copy, Clone)]
struct MouseState {
    pos: (f64, f64),
    pressed: (bool, bool, bool),
    wheel: (f32, f32),
}

impl MouseState {
    fn new() -> MouseState {
        MouseState {
            pos: (0.0, 0.0),
            pressed: (false, false, false),
            wheel: (0.0, 0.0),
        }
    }
}

fn main() -> Result<()> {
    let mut last_frame = Instant::now();
    let mut events_loop = winit::EventsLoop::new();

    let display = {
        let icon_data = include_bytes!("../32.png");

        let context = glium::glutin::ContextBuilder::new()
            .with_gl_profile(glium::glutin::GlProfile::Core)
            .with_multisampling(8)
            .with_vsync(false)
            .with_gl(glium::glutin::GlRequest::Specific(
                glium::glutin::Api::OpenGl,
                (4, 3),
            ));
        let window = winit::WindowBuilder::new()
            .with_title("UdpTest")
            .with_dimensions(LogicalSize::new(800.0, 334.0))
            .with_window_icon(Some(winit::Icon::from_bytes(icon_data).unwrap()));
        Display::new(window, context, &events_loop).unwrap()
    };

    let gl_window = display.gl_window();
    let window = gl_window.window();

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);
    {
        let style = imgui.style_mut();
        style.alpha = 1.0;
        style.frame_padding = [6.0, 4.0];
        style.frame_rounding = 3.0;
        style.grab_rounding = 3.0;
        style.scrollbar_rounding = 2.0;
        style.window_padding = [12.0, 12.0];
        style.window_rounding = 0.0;
        style.indent_spacing = 10.0;
    }

    {
        let font_data = include_bytes!("../DroidSans.ttf");

        let mut font_atlas = imgui.fonts();
        font_atlas.add_font(&[FontSource::TtfData {
            data: font_data,
            size_pixels: 16.0,
            config: None,
        }]);
    }

    let mut platform = WinitPlatform::init(&mut imgui);
    platform.attach_window(imgui.io_mut(), window, HiDpiMode::Default);

    let mut renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

    let mut state = State::new(5005, "0.0.0.0".to_owned());

    loop {
        state.last_mouse_state = state.mouse_state;
        state.mouse_state.wheel = (0.0, 0.0);

        let mut new_absolute_mouse_pos = None;

        events_loop.poll_events(|event| {
            use winit::{
                DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent,
            };

            platform.handle_event(imgui.io_mut(), &window, &event);

            match event {
                Event::DeviceEvent { event, .. } => match event {
                    DeviceEvent::MouseMotion { delta: (x, y), .. } => {
                        state.mouse_state.pos.0 += x / window.get_hidpi_factor();
                        state.mouse_state.pos.1 += y / window.get_hidpi_factor();
                    }
                    _ => (),
                },
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        state.quit = true;
                    }
                    WindowEvent::Resized(logical_size) => {
                        display
                            .gl_window()
                            .resize(logical_size.to_physical(window.get_hidpi_factor()));
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        use glium::glutin::VirtualKeyCode as Key;

                        let pressed = input.state == ElementState::Pressed;

                        match input.virtual_keycode {
                            Some(Key::Return) => {
                                if pressed && input.modifiers.alt {
                                    let monitor = if state.fullscreen {
                                        None
                                    } else {
                                        Some(window.get_current_monitor())
                                    };
                                    window.set_fullscreen(monitor);
                                    state.fullscreen = !state.fullscreen;
                                }
                            }
                            Some(Key::Space) => {
                                if pressed {
                                    state.auto_pan = !state.auto_pan;
                                }
                            }
                            _ => {}
                        }
                    }
                    WindowEvent::CursorMoved {
                        position: LogicalPosition { x, y },
                        ..
                    } => {
                        new_absolute_mouse_pos = Some((x, y));
                    }
                    WindowEvent::MouseInput {
                        state: mouse_state,
                        button,
                        ..
                    } => match button {
                        MouseButton::Left => {
                            state.mouse_state.pressed.0 = mouse_state == ElementState::Pressed
                        }
                        MouseButton::Right => {
                            state.mouse_state.pressed.1 = mouse_state == ElementState::Pressed
                        }
                        MouseButton::Middle => {
                            state.mouse_state.pressed.2 = mouse_state == ElementState::Pressed
                        }
                        _ => {}
                    },
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::PixelDelta(LogicalPosition { x, y }),
                        ..
                    } => {
                        if x != 0.0 {
                            state.mouse_state.wheel.0 = x as f32 * 50.0;
                        }
                        if y != 0.0 {
                            state.mouse_state.wheel.1 = y as f32;
                        }
                    }
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(x, y),
                        ..
                    } => {
                        if x != 0.0 {
                            state.mouse_state.wheel.0 = x * 50.0;
                        }
                        if y != 0.0 {
                            state.mouse_state.wheel.1 = y;
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        });

        if !state.mouse_state.pressed.0 {
            if let Some(pos) = new_absolute_mouse_pos {
                state.mouse_state.pos = pos;
            }
        }

        {
            imgui.io_mut().mouse_pos = [
                state.mouse_state.pos.0 as f32,
                state.mouse_state.pos.1 as f32,
            ];

            imgui.io_mut().mouse_down = [
                state.mouse_state.pressed.0,
                state.mouse_state.pressed.1,
                state.mouse_state.pressed.2,
                false,
                false,
            ];

            imgui.io_mut().mouse_wheel = state.mouse_state.wheel.1;
        }

        platform
            .prepare_frame(imgui.io_mut(), &window)
            .map_err(|_| anyhow!("Failed to prepare frame"))?;
        last_frame = imgui.io_mut().update_delta_time(last_frame);

        {
            let ui = imgui.frame();
            draw_gui(&ui, &mut state);

            let mut target = display.draw();
            target.clear_color(0.12, 0.12, 0.12, 1.0);
            renderer
                .render(&mut target, ui.render())
                .expect("Rendering failed");
            target.finish().unwrap();
        }

        if state.quit {
            break;
        }

        {
            while (last_frame.elapsed() + Duration::from_micros(2500)).as_secs_f64() < 1.0 / 60.0 {
                thread::sleep(Duration::from_millis(1));
            }

            while last_frame.elapsed().as_secs_f64() < 1.0 / 60.0 {
                thread::yield_now();
            }
        }
    }

    let args: Vec<String> = env::args().collect();

    let mut print_usage_instructions = args.len() != 3;

    let send_interval_us = 1000;
    let packet_size_bytes = 500;

    if !print_usage_instructions {
        let mode: &str = &args[1];

        match mode.as_ref() {
            "tx" => {
                let destination = &args[2];

                println!("sending to {}", destination);

                let socket = UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind to address");
                socket.connect(destination).expect("connection failed");
                let begin = Instant::now();
                let mut next_action_time_ms = 1;

                let mut buf: Vec<u8> = Vec::new();
                buf.resize(packet_size_bytes, 0);

                loop {
                    if Instant::now().saturating_duration_since(begin)
                        > Duration::from_millis(next_action_time_ms)
                    {
                        println!(
                            "Socket send took too much time! ({} > 1000)",
                            Instant::now().saturating_duration_since(begin).as_micros()
                        );
                    }

                    while Instant::now().saturating_duration_since(begin)
                        < Duration::from_millis(next_action_time_ms)
                    {}

                    next_action_time_ms += 1;

                    socket.send(&buf)?;
                }
            }
            "rx" => {
                let listen_port = args[2]
                    .parse::<u16>()
                    .expect("Failed to parse destination port");

                let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], listen_port)))?;
                socket
                    .set_read_timeout(None)
                    .expect("set_read_timeout call failed");

                let mut buf = [0; 9000];

                let mut last_rx_time = Instant::now();
                let begin = Instant::now();
                loop {
                    let (number_of_bytes, src_addr) = socket.recv_from(&mut buf)?;

                    let now = Instant::now();
                    println!(
                        "{};{};{};{}",
                        now.saturating_duration_since(begin).as_nanos(),
                        now.saturating_duration_since(last_rx_time).as_nanos(),
                        number_of_bytes,
                        src_addr
                    );
                    last_rx_time = now;
                }
            }
            &_ => {
                print_usage_instructions = true;
            }
        }
    }

    if print_usage_instructions {
        println!(
            "This program will either send a {} b udp packet every {} μs or listen for packets and print the time diff.
To use, supply arguments: tx [target_ip:port] or: rx [listen_port]", packet_size_bytes, send_interval_us);
    }

    Ok(())
}
