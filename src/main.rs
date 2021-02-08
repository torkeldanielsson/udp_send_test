#![windows_subsystem = "windows"]

use core::cmp;
use glium::glutin;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::{Display, Surface};
use imgui::*;
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::time::Instant;
use std::{path::Path, sync::Arc};

mod rx;
use rx::Rx;
mod tx;
use tx::Tx;

struct System {
    pub event_loop: EventLoop<()>,
    pub display: glium::Display,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
}

impl System {
    pub fn main_loop<F: FnMut(&mut bool, &mut Ui) + 'static>(self, mut run_ui: F) {
        let System {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;
        let mut last_frame = Instant::now();

        event_loop.run(move |event, _, control_flow| match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;
            }
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();
                platform
                    .prepare_frame(imgui.io_mut(), gl_window.window())
                    .expect("Failed to prepare frame");
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let mut ui = imgui.frame();

                let mut run = true;
                run_ui(&mut run, &mut ui);
                if !run {
                    *control_flow = ControlFlow::Exit;
                }

                let gl_window = display.gl_window();
                let mut target = display.draw();
                target.clear_color_srgb(1.0, 1.0, 1.0, 1.0);
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("Rendering failed");
                target.finish().expect("Failed to swap buffers");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            event => {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        })
    }
}

fn init(title: &str) -> System {
    let title = match Path::new(&title).file_name() {
        Some(file_name) => file_name.to_str().unwrap(),
        None => title,
    };
    let event_loop = EventLoop::new();
    let context = glutin::ContextBuilder::new().with_vsync(true);
    let builder = WindowBuilder::new()
        .with_title(title.to_owned())
        .with_inner_size(glutin::dpi::LogicalSize::new(1000f64, 200f64));
    let display =
        Display::new(builder, context, &event_loop).expect("Failed to initialize display");

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    let mut platform = WinitPlatform::init(&mut imgui);
    {
        let gl_window = display.gl_window();
        let window = gl_window.window();
        platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);
    }

    let hidpi_factor = platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.fonts().add_font(&[
        FontSource::TtfData {
            data: include_bytes!("../DroidSans.ttf"),
            size_pixels: font_size,
            config: Some(FontConfig {
                size_pixels: font_size,
                glyph_ranges: FontGlyphRanges::default(),
                ..FontConfig::default()
            }),
        },
        FontSource::TtfData {
            data: include_bytes!("../mplus-1p-regular.ttf"),
            size_pixels: font_size,
            config: Some(FontConfig {
                rasterizer_multiply: 1.75,
                glyph_ranges: FontGlyphRanges::japanese(),
                ..FontConfig::default()
            }),
        },
    ]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");

    System {
        event_loop,
        display,
        imgui,
        platform,
        renderer,
        font_size,
    }
}

fn main() {
    let system = init("Voysys UDP Test");

    let mut rx_im_string_bind_ip = ImString::new("0.0.0.0");
    rx_im_string_bind_ip.reserve(128);

    let mut rx_listen_port = 27000;

    let mut rx: Arc<Option<Rx>> =
        Arc::new(Rx::new(rx_im_string_bind_ip.as_ref(), rx_listen_port).ok());

    let mut tx_target_port = 27000;
    let mut tx_packet_size = 500;
    let mut tx_send_interval_us = 10000;

    let mut tx_im_string_target_ip = ImString::new("127.0.0.1");
    tx_im_string_target_ip.reserve(128);

    let mut tx_im_string_bind_ip = ImString::new("0.0.0.0");
    tx_im_string_bind_ip.reserve(128);

    let mut tx: Arc<Option<Tx>> = Arc::new(
        Tx::new(
            tx_im_string_bind_ip.as_ref(),
            tx_im_string_target_ip.as_ref(),
            tx_target_port,
            tx_packet_size,
            tx_send_interval_us,
        )
        .ok(),
    );

    let mut stat_length_s: f32 = 5.0;

    system.main_loop(move |_, ui| {
        let view_size = ui.io().display_size;

        let style_colors = ui.push_style_colors(&[
            (StyleColor::Text, [0.98, 0.66, 0.3, 1.0]),
            (StyleColor::CheckMark, [0.98, 0.66, 0.3, 1.0]),
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

        Window::new(im_str!("UDP Test: Tx"))
            .size([400.0, view_size[1]], Condition::Always)
            .position([0.0, 0.0], Condition::Always)
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .focused(false)
            .build(ui, || {
                ui.text(im_str!("UDP Test: Tx"));
                ui.separator();

                let mut stop = false;
                let mut start = false;

                match tx.as_ref() {
                    Some(tx) => {
                        if ui.small_button(im_str!("Stop")) {
                            stop = true;
                        }

                        ui.text(format!(
                            "Bind IP: {}",
                            tx_im_string_bind_ip.as_ref() as &str
                        ));
                        ui.text(format!(
                            "Destination IP: {}",
                            tx_im_string_target_ip.as_ref() as &str
                        ));
                        ui.text(format!("Destination Port: {}", tx_target_port));
                        ui.text(format!("Packet Size: {}", tx_packet_size));
                        ui.text(format!("Send Interval: {} µs", tx_send_interval_us));

                        ui.text(format!("Sent packets: {}", tx.get_send_count()));
                    }
                    None => {
                        if ui.small_button(im_str!("Start")) {
                            start = true;
                        }
                        ui.input_text(im_str!("Bind IP"), &mut tx_im_string_bind_ip)
                            .build();
                        ui.input_text(im_str!("Destination IP"), &mut tx_im_string_target_ip)
                            .build();
                        Drag::new(im_str!("Destination Port"))
                            .range(1..=65236)
                            .build(ui, &mut tx_target_port);
                        Drag::new(im_str!("Packet Size"))
                            .range(64..=1400)
                            .build(ui, &mut tx_packet_size);
                        Drag::new(im_str!("Send Interval"))
                            .range(500..=1000000)
                            .display_format(im_str!("%d µs"))
                            .build(ui, &mut tx_send_interval_us);
                    }
                }

                if stop {
                    tx = Arc::new(None);
                }

                if start {
                    tx = Arc::new(
                        Tx::new(
                            tx_im_string_bind_ip.as_ref(),
                            tx_im_string_target_ip.as_ref(),
                            tx_target_port,
                            tx_packet_size,
                            tx_send_interval_us,
                        )
                        .ok(),
                    );
                }
            });

        let rx_window_width = if view_size[0] > 401.0 {
            view_size[0] - 400.0
        } else {
            1.0
        };

        Window::new(im_str!("UDP Test: Rx"))
            .size([rx_window_width, view_size[1]], Condition::Always)
            .position([400.0, 0.0], Condition::Always)
            .movable(false)
            .resizable(false)
            .title_bar(false)
            .collapsible(false)
            .menu_bar(false)
            .focused(false)
            .build(ui, || {
                ui.text(im_str!("UDP Test: Rx"));
                ui.separator();

                let mut stop = false;
                let mut start = false;

                match rx.as_ref() {
                    Some(rx) => {
                        if ui.small_button(im_str!("Stop")) {
                            stop = true;
                        }

                        ui.text(format!(
                            "Bind IP: {}",
                            rx_im_string_bind_ip.as_ref() as &str
                        ));
                        ui.text(format!("Listen Port: {}", rx_listen_port));

                        Drag::new(im_str!("Statistics Window Length"))
                            .range(0.1..=1000.0)
                            .display_format(im_str!("%.02f s"))
                            .speed(0.01)
                            .build(ui, &mut stat_length_s);

                        {
                            let t_diff_data = rx.get_t_diff_data();
                            let t_rx_data = rx.get_t_rx_data();

                            if t_diff_data.len() > 2 && t_rx_data.len() > 2 {
                                let last_time = t_rx_data.last().unwrap();
                                let start_window_time = last_time - stat_length_s as f64;

                                let mut first_sample = t_rx_data.len() - 2;
                                while first_sample != 0
                                    && t_rx_data[first_sample] > start_window_time
                                {
                                    first_sample -= 1;
                                }

                                let start_window_time = t_rx_data[first_sample];
                                let end_window_time = t_rx_data.last().unwrap();
                                let window_time = end_window_time - start_window_time;

                                let sample_count = t_rx_data.len() - first_sample as usize;
                                ui.text(format!(
                                    "Rx packets in statistics range: {}",
                                    sample_count as i64
                                ));

                                let t_rx_data = &t_rx_data[first_sample..];
                                let t_diff_data = &t_diff_data[first_sample..];

                                ui.plot_lines(im_str!("Delta Times"), t_diff_data)
                                    .scale_min(0.0)
                                    .build();

                                {
                                    let mut average = 0.0;
                                    let mut min = std::f32::MAX;
                                    let mut max = std::f32::MIN;

                                    for v in t_diff_data {
                                        average += v;
                                        if v < &min {
                                            min = *v;
                                        }
                                        if v > &max {
                                            max = *v;
                                        }
                                    }

                                    average = average / t_diff_data.len() as f32;

                                    ui.text(format!(
                                        "Min: {:.02}, Max: {:.02}, Average: {:.02} (ms)",
                                        1000.0 * min,
                                        1000.0 * max,
                                        1000.0 * average
                                    ));
                                }

                                let time_samples = cmp::min(100, sample_count / 10);
                                let time_samples_dt = window_time / time_samples as f64;
                                let mut sample_start_i: usize = 0;
                                let mut sample_end_i: usize = 0;
                                let mut time_sample_time_i = start_window_time;
                                let mut time_samples_data = Vec::new();
                                for _ in 0..time_samples {
                                    sample_start_i = sample_end_i;
                                    time_sample_time_i += time_samples_dt;
                                    while sample_end_i < sample_count
                                        && t_rx_data[sample_end_i] < time_sample_time_i
                                    {
                                        sample_end_i += 1;
                                    }
                                    time_samples_data.push((sample_end_i - sample_start_i) as f32);
                                }
                                ui.plot_lines(
                                    im_str!("Packets Per Time"),
                                    time_samples_data.as_slice(),
                                )
                                .scale_min(0.0)
                                .build();
                            }
                        }

                        ui.spacing();
                    }
                    None => {
                        if ui.small_button(im_str!("Start")) {
                            start = true;
                        }
                        ui.input_text(im_str!("Bind IP"), &mut rx_im_string_bind_ip)
                            .build();
                        Drag::new(im_str!("Listen Port"))
                            .range(1..=65236)
                            .build(ui, &mut rx_listen_port);
                    }
                }

                if stop {
                    rx = Arc::new(None);
                }

                if start {
                    rx = Arc::new(Rx::new(tx_im_string_bind_ip.as_ref(), rx_listen_port).ok());
                    if !rx.is_some() {
                        println!("Failed to open");
                    }
                }
            });

        style_colors.pop(&ui);
    });
}
