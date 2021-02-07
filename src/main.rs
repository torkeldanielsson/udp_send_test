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
        .with_inner_size(glutin::dpi::LogicalSize::new(1024f64, 768f64));
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

    let mut im_string_bind_ip = ImString::new("0.0.0.0");
    im_string_bind_ip.reserve(128);

    let mut rx_bind_port = 27000;

    //let mut rx = Rx::new(im_string_bind_ip.as_ref(), rx_bind_port);

    let mut tx_target_port = 27000;
    let mut tx_packet_size = 500;
    let mut tx_send_interval_us = 1000;

    let mut im_string_target_ip = ImString::new("127.0.0.1");
    im_string_target_ip.reserve(128);

    let mut im_string_bind_ip = ImString::new("0.0.0.0");
    im_string_bind_ip.reserve(128);

    let mut tx: Arc<Option<Tx>> = Arc::new(
        Tx::new(
            im_string_bind_ip.as_ref(),
            im_string_target_ip.as_ref(),
            tx_target_port,
            tx_packet_size,
            tx_send_interval_us,
        )
        .ok(),
    );

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

                        ui.text(format!("Bind IP: {}", im_string_bind_ip.as_ref() as &str));
                        ui.text(format!(
                            "Destination IP: {}",
                            im_string_target_ip.as_ref() as &str
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
                        ui.input_text(im_str!("Bind IP"), &mut im_string_bind_ip)
                            .build();
                        ui.input_text(im_str!("Destination IP"), &mut im_string_target_ip)
                            .build();
                        Drag::new(im_str!("Destination Port"))
                            .range(1..=65236)
                            .build(ui, &mut tx_target_port);
                        Drag::new(im_str!("Packet Size"))
                            .range(64..=1400)
                            .build(ui, &mut tx_packet_size);
                        Drag::new(im_str!("Send Interval (time between packets)"))
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
                            im_string_bind_ip.as_ref(),
                            im_string_target_ip.as_ref(),
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
            });

        style_colors.pop(&ui);
    });
}
