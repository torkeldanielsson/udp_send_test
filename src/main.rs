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
use std::{path::Path, sync::mpsc};

mod link;

use link::{Link, LinkMode, LinkPacketData};

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
    let system = init("UDP Test");

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

    //let (tx, rx) = mpsc::channel();

    let mut tx_link: Option<Link> = Option::None;
    let mut rx_link: Option<Link> = Option::None;
    /*
    match Link::new(
        LinkMode::Tx,
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
    }*/

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
