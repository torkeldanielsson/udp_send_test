#![windows_subsystem = "windows"]

mod link;

use link::{Link, LinkMode, LinkPacketData};

use anyhow::{anyhow, Result};
use glium::{Display, Surface};
use image;
use imgui::*;
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    platform::desktop::EventLoopExtDesktop,
};

#[derive(Debug)]
struct State {
    mouse_state: MouseState,
    last_mouse_state: MouseState,

    pan: [f64; 2],
    panning: bool,
    auto_pan: bool,
    scroll_factor: f64,

    alt_pressed: bool,

    quit: bool,
    fullscreen: bool,

    link_ok: bool,
    link: Link,
    link_mode: LinkMode,
    target_port: i32,
    target_ip: ImString,
    bind_port: i32,
    bind_ip: ImString,
    packet_size: i32,
    rx: mpsc::Receiver<LinkPacketData>,
    send_interval_us: i32,

    link_data: Vec<LinkPacketData>,

    difference_data: Vec<f64>,
    largest_diff: f64,
}

impl State {
    fn new() -> State {
        // defaults:
        let target_port = 5005;
        let target_ip = "127.0.0.1";
        let bind_port = target_port;
        let bind_ip = "0.0.0.0";
        let mut link_mode = LinkMode::Rx;
        let packet_size = 500;
        let send_interval_us = 1000;

        let mut im_string_target_ip = ImString::new(target_ip);
        im_string_target_ip.reserve(128);

        let mut im_string_bind_ip = ImString::new(bind_ip);
        im_string_bind_ip.reserve(128);

        let (tx, rx) = mpsc::channel();

        let link;
        match Link::new(
            link_mode.clone(),
            &bind_ip,
            bind_port as u16,
            &target_ip,
            target_port as u16,
            packet_size,
            tx.clone(),
            send_interval_us,
        ) {
            Ok(new_link) => {
                link = new_link;
            }
            Err(_) => {
                link_mode = LinkMode::Rx;
                link = Link::new(
                    link_mode.clone(),
                    &bind_ip,
                    0,
                    &target_ip,
                    target_port as u16,
                    packet_size,
                    tx,
                    send_interval_us,
                )
                .expect("Failed to create send link too");
            }
        }

        State {
            mouse_state: MouseState::new(),
            last_mouse_state: MouseState::new(),
            pan: [0.0, 0.0],
            panning: false,
            auto_pan: true,
            scroll_factor: 2.2,
            alt_pressed: false,
            quit: false,
            fullscreen: false,
            link_ok: true,
            link: link,
            link_mode: link_mode,
            target_port: target_port,
            target_ip: im_string_target_ip,
            bind_port: bind_port,
            bind_ip: im_string_bind_ip,
            packet_size: packet_size,
            rx: rx,
            send_interval_us: send_interval_us,
            link_data: Vec::new(),
            difference_data: Vec::new(),
            largest_diff: 0.0,
        }
    }
}

fn draw_gui(ui: &Ui, state: &mut State, platform_window: &winit::window::Window) {
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

                    state.pan[0] += dx * platform_window.scale_factor() as f64;
                    state.pan[1] += dy * platform_window.scale_factor() as f64;

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

            if view_size[0] > 400.1 && state.link_data.len() > 2 {
                let draw_list = ui.get_window_draw_list();

                let mut x = 400.0;

                let t_data_min = state.link_data[0].t;
                let n = state.link_data.len() - 1;
                let t_data_max = state.link_data[n].t;
                let t_data_diff = t_data_max - t_data_min;

                let mut index = 1;
                let mut diff = state.link_data[1].t - state.link_data[0].t;

                while x < view_size[0] {
                    let t: f64 = ((x - 400.0) / (view_size[0] - 400.0)) as f64;

                    let current_data_t = t_data_min + t * t_data_diff;

                    if current_data_t >= state.link_data[index].t && current_data_t < t_data_max {
                        diff = 0.0;

                        while state.link_data[index].t < current_data_t {
                            let tmp_diff = state.link_data[index + 1].t - state.link_data[index].t;
                            if tmp_diff > diff {
                                diff = tmp_diff;
                            }
                            index += 1;
                        }
                    }

                    draw_list
                        .add_line(
                            [x, view_size[1]],
                            [
                                x,
                                view_size[1] - ((diff / state.largest_diff) as f32) * view_size[1],
                            ],
                            [0.2, 0.65, 0.3, 1.0],
                        )
                        .build();

                    x += 1.0;
                }
            }

            {
                let style_colors = ui.push_style_colors(&[
                    (
                        StyleColor::Text,
                        [0.980000019, 0.664439976, 0.303800017, 1.0],
                    ),
                    (
                        StyleColor::CheckMark,
                        [0.980000019, 0.664439976, 0.303800017, 1.0],
                    ),
                    (StyleColor::WindowBg, [0.2, 0.2, 0.2, 1.0]),
                    (StyleColor::FrameBg, [1.0, 1.0, 1.0, 0.14]),
                    (StyleColor::SliderGrabActive, [0.9, 0.9, 0.9, 0.80]),
                    (StyleColor::PopupBg, [0.2, 0.2, 0.2, 1.0]),
                    (StyleColor::ScrollbarBg, [0.2, 0.2, 0.2, 1.0]),
                    (StyleColor::TitleBg, [0.2, 0.2, 0.2, 1.0]),
                    (StyleColor::TitleBgActive, [0.2, 0.2, 0.2, 1.0]),
                    (StyleColor::TitleBgCollapsed, [0.2, 0.2, 0.2, 1.0]),
                    (StyleColor::MenuBarBg, [0.165, 0.165, 0.165, 1.0]),
                    (StyleColor::Border, [0.314, 0.314, 0.314, 1.0]),
                    (StyleColor::BorderShadow, [0.0, 0.0, 0.0, 0.0]),
                    (StyleColor::SliderGrab, [0.6, 0.6, 0.6, 0.8]),
                    (StyleColor::SliderGrabActive, [0.9, 0.9, 0.9, 0.8]),
                    (StyleColor::ScrollbarGrab, [0.6, 0.6, 0.6, 0.8]),
                    (StyleColor::ScrollbarGrabActive, [0.9, 0.9, 0.9, 0.8]),
                    (StyleColor::ScrollbarGrabHovered, [0.6, 0.6, 0.6, 0.8]),
                    (StyleColor::Header, [0.15, 0.15, 0.15, 0.8]),
                    (StyleColor::HeaderActive, [0.4, 0.27, 0.13, 1.0]),
                    (StyleColor::HeaderHovered, [0.3, 0.2, 0.09, 1.0]),
                    (StyleColor::Button, [0.15, 0.15, 0.15, 0.8]),
                    (StyleColor::ButtonActive, [0.4, 0.27, 0.13, 1.0]),
                    (StyleColor::ButtonHovered, [0.3, 0.2, 0.09, 1.0]),
                    (StyleColor::FrameBgActive, [0.4, 0.27, 0.13, 1.0]),
                    (StyleColor::FrameBgHovered, [0.3, 0.2, 0.09, 1.0]),
                ]);

                Window::new(im_str!("Properties"))
                    .size([400.0, view_size[1]], Condition::Always)
                    .position([0.0, 0.0], Condition::Always)
                    .movable(false)
                    .resizable(false)
                    .title_bar(false)
                    .collapsible(false)
                    .focused(true)
                    .build(ui, || {
                        {
                            // Manage connection

                            {
                                // OMG, why does combo boxes not work?!
                                // if let Some(combo_token) = ComboBox::new(im_str!("Mode")).begin(ui) {
                                //     if Selectable::new(im_str!("Tx"))
                                //         .selected(state.link_mode == LinkMode::Tx)
                                //         .build(ui)
                                //     {
                                //         state.link_mode = LinkMode::Tx;
                                //     }
                                //     if Selectable::new(im_str!("Rx"))
                                //         .selected(state.link_mode == LinkMode::Rx)
                                //         .build(ui)
                                //     {
                                //         state.link_mode = LinkMode::Rx;
                                //     }
                                //     combo_token.end(ui);
                                // }

                                match state.link_mode {
                                    LinkMode::Tx => {
                                        ui.text(im_str!("Mode: Tx"));
                                        if ui.small_button(im_str!("Change to Rx")) {
                                            state.link_mode = LinkMode::Rx;
                                        }
                                    }
                                    LinkMode::Rx => {
                                        ui.text(im_str!("Mode: Rx"));
                                        if ui.small_button(im_str!("Change to Tx")) {
                                            state.link_mode = LinkMode::Tx;
                                        }
                                    }
                                }
                            }

                            ui.input_text(im_str!("Bind IP"), &mut state.bind_ip)
                                .build();

                            match state.link_mode {
                                LinkMode::Tx => {
                                    ui.input_text(im_str!("Destination IP"), &mut state.target_ip)
                                        .build();
                                    ui.drag_int(
                                        im_str!("Destination Port"),
                                        &mut state.target_port,
                                    )
                                    .min(1)
                                    .max(65535)
                                    .build();
                                }
                                LinkMode::Rx => {
                                    ui.drag_int(im_str!("Bind Port"), &mut state.bind_port)
                                        .min(1)
                                        .max(65535)
                                        .build();
                                }
                            }

                            let address_and_port_ok = match state.link_mode {
                                LinkMode::Tx => {
                                    state.target_ip.to_str() == state.link.target_address
                                        && state.target_port as u16 == state.link.target_port
                                        && state.bind_ip.to_str() == state.link.bind_address
                                }
                                LinkMode::Rx => {
                                    state.bind_ip.to_str() == state.link.bind_address
                                        && state.bind_port as u16 == state.link.bind_port
                                }
                            };

                            if !state.link_ok
                                || state.link_mode != state.link.link_mode
                                || !address_and_port_ok
                                || state.packet_size != state.link.packet_size
                                || state.send_interval_us != state.link.send_interval_us
                            {
                                state.link.run.store(false, Ordering::SeqCst);
                                if let Some(thread) = state.link.thread.take() {
                                    thread.join().ok();
                                }

                                let (tx, rx) = mpsc::channel();

                                state.rx = rx;
                                state.link_ok = false;
                                state.link_data = Vec::new();
                                state.difference_data = Vec::new();
                                state.largest_diff = 0.0;

                                let bind_port = match state.link_mode {
                                    LinkMode::Tx => 0,

                                    LinkMode::Rx => state.bind_port,
                                };

                                match (
                                    IpAddr::from_str(state.bind_ip.to_str()),
                                    IpAddr::from_str(state.target_ip.to_str()),
                                ) {
                                    (Ok(_), Ok(_)) => {
                                        match Link::new(
                                            state.link_mode.clone(),
                                            state.bind_ip.to_str(),
                                            bind_port as u16,
                                            state.target_ip.to_str(),
                                            state.target_port as u16,
                                            state.packet_size,
                                            tx,
                                            state.send_interval_us,
                                        ) {
                                            Ok(link) => {
                                                state.link = link;
                                                state.link_ok = true;
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        {
                            // Show Link stats

                            match state.link_mode {
                                LinkMode::Tx => {
                                    let im_string = ImString::new(format!(
                                        "Sent Packets: {}",
                                        state.link_data.len()
                                    ));
                                    ui.text(&im_string);
                                }
                                LinkMode::Rx => {
                                    {
                                        let im_string = ImString::new(format!(
                                            "Received Packets: {}",
                                            state.link_data.len()
                                        ));
                                        ui.text(&im_string);
                                    }
                                    {
                                        let im_string = ImString::new(format!(
                                            "Max diff: {}",
                                            state.largest_diff
                                        ));
                                        ui.text(&im_string);
                                    }
                                }
                            }
                        }
                    });

                style_colors.pop(&ui);
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
    let mut event_loop = winit::event_loop::EventLoop::new();

    let display = {
        let icon_data = image::load_from_memory(include_bytes!("../32.png"))
            .unwrap()
            .into_rgba();

        let context = glium::glutin::ContextBuilder::new()
            .with_gl_profile(glium::glutin::GlProfile::Core)
            .with_multisampling(8)
            .with_vsync(false)
            .with_gl(glium::glutin::GlRequest::Specific(
                glium::glutin::Api::OpenGl,
                (4, 3),
            ));
        let window = winit::window::WindowBuilder::new()
            .with_title("Udp Test")
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_window_icon(Some(
                winit::window::Icon::from_rgba(
                    (&*icon_data).to_owned(),
                    icon_data.width(),
                    icon_data.height(),
                )
                .unwrap(),
            ));
        Display::new(window, context, &event_loop).unwrap()
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

    let mut state = State::new();

    loop {
        if state.link_ok {
            loop {
                match state.rx.recv_timeout(Duration::from_millis(0)) {
                    Ok(msg) => {
                        // println!("msg: {:?}", msg);
                        state.link_data.push(msg);
                        if state.link_data.len() >= 2 {
                            let last_two: Vec<&LinkPacketData> =
                                state.link_data.iter().rev().take(2).collect();
                            let diff = last_two[0].t - last_two[1].t;
                            state.difference_data.push(diff);
                            if state.largest_diff < diff {
                                state.largest_diff = diff;
                            }
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        }

        state.last_mouse_state = state.mouse_state;
        state.mouse_state.wheel = (0.0, 0.0);

        let mut new_absolute_mouse_pos = None;

        let mut change_fullscreen = false;
        let mut new_fullscreen_mode = None;

        event_loop.run_return(|event, _, control_flow| {
            use winit::event::{
                DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta,
                WindowEvent,
            };

            *control_flow = winit::event_loop::ControlFlow::Exit;

            platform.handle_event(imgui.io_mut(), &window, &event);

            match event {
                Event::DeviceEvent { event, .. } => match event {
                    DeviceEvent::MouseMotion {
                        delta: (px, py), ..
                    } => {
                        let LogicalPosition::<f64> { x: lx, y: ly } =
                            PhysicalPosition { x: px, y: py }.to_logical(window.scale_factor());

                        state.mouse_state.pos.0 += lx;
                        state.mouse_state.pos.1 += ly;
                    }
                    DeviceEvent::ModifiersChanged(modifiers_changed) => {
                        state.alt_pressed = modifiers_changed.alt()
                    }
                    _ => (),
                },
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        state.quit = true;
                    }
                    WindowEvent::Resized(physical_size) => {
                        display.gl_window().resize(physical_size);
                    }
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input:
                            KeyboardInput {
                                state: key_state,
                                virtual_keycode,
                                ..
                            },
                        is_synthetic: _,
                    } => {
                        use winit::event::VirtualKeyCode as Key;

                        let pressed = key_state == ElementState::Pressed;

                        match virtual_keycode {
                            Some(Key::Return) => {
                                if pressed && state.alt_pressed {
                                    let monitor = if state.fullscreen {
                                        None
                                    } else {
                                        Some(window.current_monitor())
                                    };

                                    new_fullscreen_mode = monitor.map(|monitor| {
                                        winit::window::Fullscreen::Borderless(monitor)
                                    });
                                    change_fullscreen = true;

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
                    WindowEvent::CursorMoved { position, .. } => {
                        let LogicalPosition { x, y } = position.to_logical(window.scale_factor());
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

        if change_fullscreen {
            window.set_fullscreen(new_fullscreen_mode);
        }

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
            draw_gui(&ui, &mut state, &window);

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

    state.link.run.store(false, Ordering::SeqCst);
    if let Some(thread) = state.link.thread.take() {
        thread.join().ok();
    }

    Ok(())
}
