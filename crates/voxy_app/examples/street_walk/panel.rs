//! Left-hand settings panel (F1 toggles). It takes input only while the
//! mouse is released (Esc), so it never fights the walk camera.
use crate::day_cycle::DayCycle;
use voxy_render::climate::Season;
use voxy_render::weather::Weather;
use winit::window::Window;

/// Logical width of the left settings column; the scene renders beside it.
pub const WIDTH: f32 = 260.;

pub struct Panel {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    pub visible: bool,
}

pub struct Settings<'a> {
    pub cycle: &'a mut Option<DayCycle>,
    pub weather: &'a mut Weather,
    pub wetness: &'a mut Option<f32>,
    pub seed: u64,
    pub ageing: Option<&'a mut crate::original::Ageing>,
    /// Set when an ageing change should regenerate the street.
    pub rebuild: &'a mut bool,
}

impl Panel {
    pub fn new(window: &Window, device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let ctx = egui::Context::default();
        ctx.set_visuals(egui::Visuals::dark());
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let renderer = egui_wgpu::Renderer::new(device, format, Default::default());
        Self { ctx, state, renderer, visible: true }
    }
    /// True when egui consumed the event.
    pub fn on_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        self.visible && self.state.on_window_event(window, event).consumed
    }
    pub fn wants_pointer(&self) -> bool {
        self.visible && (self.ctx.is_pointer_over_egui() || self.ctx.egui_wants_pointer_input())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        size: [u32; 2],
        settings: Settings,
    ) -> Option<wgpu::SubmissionIndex> {
        if !self.visible {
            return None;
        }
        let input = self.state.take_egui_input(window);
        let mut settings = settings;
        let output = self.ctx.run_ui(input, |ui| {
            egui::Panel::left("street settings")
                .resizable(false)
                .exact_size(WIDTH)
                .show(ui, |ui| contents(ui, &mut settings));
        });
        self.state.handle_platform_output(window, output.platform_output);
        let jobs = self.ctx.tessellate(output.shapes, output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: size,
            pixels_per_point: output.pixels_per_point,
        };
        for (id, delta) in &output.textures_delta.set {
            for delta in delta {
                self.renderer.update_texture(device, queue, *id, delta);
            }
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("street settings panel"),
        });
        let extra = self.renderer.update_buffers(device, queue, &mut encoder, &jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("street settings panel"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &jobs, &screen);
        }
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        Some(queue.submit(extra.into_iter().chain([encoder.finish()])))
    }
}

fn contents(ui: &mut egui::Ui, s: &mut Settings) {
    ui.heading("Настройки");
    ui.label("F1 скрыть · Esc отпустить мышь");
    ui.separator();

    let mut enabled = s.cycle.is_some();
    if ui.checkbox(&mut enabled, "Смена дня, сезонов и погоды").changed() {
        *s.cycle = enabled.then(|| DayCycle::new(s.seed));
    }
    if let Some(cycle) = s.cycle.as_mut() {
        let c = &mut cycle.calendar;
        let mut hour = c.hour();
        let day = c.day();
        ui.label(format!(
            "День {} · {:02}:{:02} · {}",
            day as u32 + 1,
            hour as u32,
            (hour.fract() * 60.) as u32,
            season_name(c.season())
        ));
        if ui.add(egui::Slider::new(&mut hour, 0.0..=23.99).text("час")).changed() {
            c.hours = day * 24. + hour;
        }
        let mut day_of_year = day + 1.;
        if ui.add(egui::Slider::new(&mut day_of_year, 1.0..=365.).integer().text("день года")).changed() {
            c.hours = (day_of_year - 1.) * 24. + c.hour();
        }
        ui.horizontal(|ui| {
            for season in [Season::Spring, Season::Summer, Season::Autumn, Season::Winter] {
                if ui.selectable_label(c.season() == season, season_name(season)).clicked() {
                    c.hours = season.middle_day() * 24. + c.hour();
                }
            }
        });
        ui.add(egui::Slider::new(&mut c.latitude_degrees, -70.0..=70.).text("широта °"));
        ui.separator();
        ui.checkbox(&mut cycle.paused, "Пауза времени");
        let mut minutes = 1. / cycle.time_scale.max(1e-6) * 24. / 60.;
        if ui
            .add(egui::Slider::new(&mut minutes, 0.5..=240.).logarithmic(true).text("мин на сутки"))
            .changed()
        {
            cycle.time_scale = 24. / (minutes * 60.);
        }
        ui.separator();
        ui.checkbox(&mut cycle.auto_weather, "Погода по сезону");
    }

    ui.separator();
    ui.label("Погода");
    let before = *s.weather;
    ui.horizontal_wrapped(|ui| {
        for (name, preset) in [
            ("Ясно", Weather::CLEAR),
            ("Облачно", Weather::CLOUDY),
            ("Дождь", Weather::STORM),
            ("Снег", Weather::SNOW),
        ] {
            if ui.button(name).clicked() {
                let (wind, gust) = (s.weather.wind, s.weather.wind_gust);
                *s.weather = Weather { wind, wind_gust: gust, ..preset };
            }
        }
    });
    let w = &mut *s.weather;
    ui.add(egui::Slider::new(&mut w.coverage, 0.0..=1.).text("облачность"));
    ui.add(egui::Slider::new(&mut w.density, 0.0..=4.).text("плотность"));
    ui.add(egui::Slider::new(&mut w.rain, 0.0..=1.).text("дождь"));
    ui.add(egui::Slider::new(&mut w.snow, 0.0..=1.).text("снег"));
    let [x, z] = w.wind;
    let mut speed = x.hypot(z);
    let mut direction = z.atan2(x).to_degrees().rem_euclid(360.);
    let wind_speed = ui.add(egui::Slider::new(&mut speed, 0.0..=30.).text("ветер м/с")).changed();
    let wind_direction = ui.add(egui::Slider::new(&mut direction, 0.0..=359.).text("направление °")).changed();
    if wind_speed || wind_direction {
        let a = direction.to_radians();
        w.wind = [a.cos() * speed, a.sin() * speed];
    }
    ui.add(egui::Slider::new(&mut w.wind_gust, 0.0..=1.).text("порывы"));
    if *s.weather != before {
        if let Some(cycle) = s.cycle.as_mut() {
            cycle.auto_weather = false;
        }
    }

    ui.separator();
    let mut manual = s.wetness.is_some();
    ui.checkbox(&mut manual, "Влажность вручную");
    if manual != s.wetness.is_some() {
        *s.wetness = manual.then_some(0.5);
    }
    if let Some(wet) = s.wetness.as_mut() {
        ui.add(egui::Slider::new(wet, 0.0..=1.).text("влажность"));
    }
    if let Some(a) = s.ageing.as_deref_mut() {
        ageing(ui, a, s.rebuild);
    }
}

fn ageing(ui: &mut egui::Ui, a: &mut crate::original::Ageing, rebuild: &mut bool) {
    ui.separator();
    ui.label("Старение улицы");
    let mut slider = |ui: &mut egui::Ui, value: &mut f32, text: &str| {
        let r = ui.add(egui::Slider::new(value, 0.0..=2.).text(text));
        // Regenerating takes a moment, so wait for the drag to end.
        if r.drag_stopped() || (r.changed() && !r.dragged()) {
            *rebuild = true;
        }
    };
    slider(ui, &mut a.damage, "повреждения");
    slider(ui, &mut a.colour, "разброс цвета");
    slider(ui, &mut a.relaid, "перемощено");
    slider(ui, &mut a.dirt, "грязь и трещины");
    ui.horizontal(|ui| {
        if ui.button("Новая").clicked() {
            *a = crate::original::Ageing { damage: 0., colour: 0., relaid: 0., dirt: 0. };
            *rebuild = true;
        }
        if ui.button("По умолчанию").clicked() {
            *a = Default::default();
            *rebuild = true;
        }
    });
}

fn season_name(season: Season) -> &'static str {
    match season {
        Season::Spring => "весна",
        Season::Summer => "лето",
        Season::Autumn => "осень",
        Season::Winter => "зима",
    }
}
