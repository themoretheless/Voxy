//! Left-hand settings panel (F1 toggles). It takes input only while the
//! mouse is released (Esc), so it never fights the walk camera.
use crate::day_cycle::DayCycle;
use voxy_render::ArchiveAntialiasing;
use voxy_render::archive_scene::ArchiveToneMapper;
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

/// Key viewpoints around the street and villa for quick teleport and cinematic auto-tour.
pub const TOUR_WAYPOINTS: [(&str, [f64; 3], [f32; 3]); 8] = [
    ("Улица", [0.6, 0.13, -18.0], [-0.10, 0.025, 1.0]),
    ("Площадь и киоски", [0.0, 0.13, 66.0], [0.0, 0.02, 1.0]),
    ("Бассейн и вилла", [-6.2, 0.05, 110.5], [0.27, -0.05, 0.96]),
    (
        "Сауна и оранжерея",
        [8.5, 0.05, 121.5],
        [0.45, -0.02, 0.89],
    ),
    (
        "Дзен-сад и бочче",
        [-8.5, 0.05, 121.5],
        [-0.45, -0.02, 0.89],
    ),
    (
        "Терраса на крыше",
        [-2.0, 6.95, 134.5],
        [0.18, -0.10, -0.98],
    ),
    (
        "Обсерватория",
        [-3.2, 10.15, 136.5],
        [-0.85, 0.12, -0.51],
    ),
    (
        "Винный грот",
        [6.2, 0.05, 101.0],
        [0.92, 0.05, 0.38],
    ),
];

pub struct Settings<'a> {
    pub cycle: &'a mut Option<DayCycle>,
    pub weather: &'a mut Weather,
    pub wetness: &'a mut Option<f32>,
    /// Street seed; editing it regenerates the street.
    pub seed_mut: Option<&'a mut u64>,
    /// Fixed sun elevation in degrees, used while the day cycle is off.
    pub sun_elevation: &'a mut Option<f32>,
    pub render: &'a mut RenderSettings,
    /// Set when a render setting should be applied to the live renderer.
    pub reconfigure: &'a mut bool,
    pub ageing: Option<&'a mut crate::original::Ageing>,
    /// Panorama sky toggle; switching it regenerates the street.
    pub hdri_sky: Option<&'a mut bool>,
    /// Set when an ageing change should regenerate the street.
    pub rebuild: &'a mut bool,
    /// Set when the user clicks the interactive water splash button.
    pub splash: &'a mut bool,
    /// Set when the user clicks the photo-mode screenshot button (F2).
    pub photo: &'a mut bool,
    /// Active state of the cinematic camera auto-tour (F3).
    pub tour: &'a mut bool,
    /// Set when the user selects a quick camera teleport viewpoint.
    pub teleport: &'a mut Option<([f64; 3], [f32; 3])>,
    /// Live frame render time in ms and optional GPU pass timings `[total, opaque, transparent]`.
    pub perf_ms: (f64, Option<[f64; 3]>),
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
        Self {
            ctx,
            state,
            renderer,
            visible: true,
        }
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
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| contents(ui, &mut settings));
                });
        });
        self.state
            .handle_platform_output(window, output.platform_output);
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
        let extra = self
            .renderer
            .update_buffers(device, queue, &mut encoder, &jobs, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("street settings panel"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
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
    if s.perf_ms.0 > 0. {
        let fps = (1000. / s.perf_ms.0).min(999.);
        if let Some([tot, opq, trn]) = s.perf_ms.1 {
            ui.label(format!(
                "Кадр {:.2} мс ({:.0} FPS) · GPU {:.2} мс (opq {:.2} · wtr {:.2})",
                s.perf_ms.0, fps, tot, opq, trn
            ));
        } else {
            ui.label(format!("Кадр {:.2} мс ({:.0} FPS)", s.perf_ms.0, fps));
        }
    }
    ui.separator();

    let mut enabled = s.cycle.is_some();
    if ui
        .checkbox(&mut enabled, "Смена дня, сезонов и погоды")
        .changed()
    {
        *s.cycle = enabled.then(|| DayCycle::new(s.seed_mut.as_deref().copied().unwrap_or(0)));
    }
    ui.horizontal_wrapped(|ui| {
        for (label, target_hour) in [
            ("Утро (F9)", 7.5_f64),
            ("Полдень (F10)", 12.5),
            ("Закат (F11)", 17.75),
            ("Ночь (F12)", 22.0),
        ] {
            if ui.button(label).clicked() {
                let seed = s.seed_mut.as_deref().copied().unwrap_or(0);
                let cycle = s.cycle.get_or_insert_with(|| DayCycle::new(seed));
                cycle.calendar.hours = cycle.calendar.day() * 24. + target_hour;
            }
        }
    });
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
        if ui
            .add(egui::Slider::new(&mut hour, 0.0..=23.99).text("час"))
            .changed()
        {
            c.hours = day * 24. + hour;
        }
        let mut day_of_year = day + 1.;
        if ui
            .add(
                egui::Slider::new(&mut day_of_year, 1.0..=365.)
                    .integer()
                    .text("день года"),
            )
            .changed()
        {
            c.hours = (day_of_year - 1.) * 24. + c.hour();
        }
        ui.horizontal(|ui| {
            for season in [
                Season::Spring,
                Season::Summer,
                Season::Autumn,
                Season::Winter,
            ] {
                if ui
                    .selectable_label(c.season() == season, season_name(season))
                    .clicked()
                {
                    c.hours = season.middle_day() * 24. + c.hour();
                }
            }
        });
        ui.add(egui::Slider::new(&mut c.latitude_degrees, -70.0..=70.).text("широта °"));
        ui.separator();
        ui.checkbox(&mut cycle.paused, "Пауза времени");
        let mut minutes = 1. / cycle.time_scale.max(1e-6) * 24. / 60.;
        if ui
            .add(
                egui::Slider::new(&mut minutes, 0.5..=240.)
                    .logarithmic(true)
                    .text("мин на сутки"),
            )
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
                *s.weather = Weather {
                    wind,
                    wind_gust: gust,
                    ..preset
                };
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
    let wind_speed = ui
        .add(egui::Slider::new(&mut speed, 0.0..=30.).text("ветер м/с"))
        .changed();
    let wind_direction = ui
        .add(egui::Slider::new(&mut direction, 0.0..=359.).text("направление °"))
        .changed();
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
    if s.cycle.is_none() {
        let mut fixed = s.sun_elevation.is_some();
        if ui.checkbox(&mut fixed, "Солнце вручную").changed() {
            *s.sun_elevation = fixed.then_some(35.);
            *s.reconfigure = true;
        }
        if let Some(elevation) = s.sun_elevation.as_mut() {
            if ui
                .add(egui::Slider::new(elevation, -10.0..=90.).text("высота солнца °"))
                .changed()
            {
                *s.reconfigure = true;
            }
        }
    }
    render(ui, s.render, s.reconfigure, s.rebuild);
    if let Some(seed) = s.seed_mut.as_deref_mut() {
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(format!("Сид улицы {seed:#x}"));
            if ui.button("Другая улица").clicked() {
                *seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                *s.rebuild = true;
            }
        });
    }
    if let Some(hdri) = s.hdri_sky.as_deref_mut() {
        ui.separator();
        if ui.checkbox(hdri, "Небо из панорамы (HDRI)").changed() {
            *s.rebuild = true;
        }
    }
    if let Some(a) = s.ageing.as_deref_mut() {
        ageing(ui, a, s.rebuild);
    }
    ui.separator();
    ui.label("Локации и интерактив");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Всплеск воды (G)").clicked() {
            *s.splash = true;
        }
        if ui.button("Скриншот без UI (F2)").clicked() {
            *s.photo = true;
        }
        ui.checkbox(s.tour, "Кино-тур по локациям (F3)");
    });
    ui.horizontal_wrapped(|ui| {
        for (name, feet, dir) in TOUR_WAYPOINTS {
            if ui.button(name).clicked() {
                *s.tour = false;
                *s.teleport = Some((feet, dir));
            }
        }
    });
}

/// Renderer options that used to be environment-only; the environment still
/// seeds the initial values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSettings {
    pub antialiasing: ArchiveAntialiasing,
    pub tone_mapper: ArchiveToneMapper,
    pub lens: Option<Lens>,
    /// Irradiance probe spacing in metres; `None` skips the bake.
    pub probe_spacing: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lens {
    pub f_number: f32,
    /// 0 = centre autofocus.
    pub focus_distance_m: f32,
    /// Fraction of the frame interval the shutter is open (0.5 = 180 degrees).
    pub shutter_fraction: f32,
}

impl RenderSettings {
    /// Reads VOXY_STREET_AA, VOXY_STREET_TONEMAP, VOXY_STREET_DOF
    /// ("5.6,auto,0.5" = f/5.6, centre autofocus, 180-degree shutter) and
    /// VOXY_STREET_PROBES ("off" or spacing in m).
    pub fn from_environment() -> Result<Self, String> {
        let var = |name: &str| std::env::var(name).ok();
        let antialiasing = match var("VOXY_STREET_AA").as_deref().unwrap_or("temporal") {
            "off" => ArchiveAntialiasing::Off,
            "spatial" => ArchiveAntialiasing::Spatial,
            "temporal" => ArchiveAntialiasing::Temporal,
            "metalfx" => ArchiveAntialiasing::MetalFx,
            value => return Err(format!("unknown VOXY_STREET_AA: {value}")),
        };
        let tone_mapper = match var("VOXY_STREET_TONEMAP").as_deref().unwrap_or("agx") {
            "aces" => ArchiveToneMapper::Aces,
            "agx" => ArchiveToneMapper::Agx,
            value => return Err(format!("unknown VOXY_STREET_TONEMAP: {value}")),
        };
        let lens = match var("VOXY_STREET_DOF").as_deref().unwrap_or("off") {
            "off" => None,
            value => {
                let parts: Vec<f32> = value
                    .split(',')
                    .filter_map(|v| match v.trim() {
                        "auto" => Some(0.),
                        v => v.parse().ok(),
                    })
                    .collect();
                if parts.len() < 2 {
                    return Err(format!("invalid VOXY_STREET_DOF: {value}"));
                }
                Some(Lens {
                    f_number: parts[0],
                    focus_distance_m: parts[1],
                    shutter_fraction: parts.get(2).copied().unwrap_or(0.),
                })
            }
        };
        let probe_spacing = match var("VOXY_STREET_PROBES").as_deref().unwrap_or("6") {
            "off" => None,
            value => Some(
                value
                    .parse()
                    .map_err(|_| format!("invalid VOXY_STREET_PROBES: {value}"))?,
            ),
        };
        Ok(Self {
            antialiasing,
            tone_mapper,
            lens,
            probe_spacing,
        })
    }
}

fn render(ui: &mut egui::Ui, r: &mut RenderSettings, reconfigure: &mut bool, rebuild: &mut bool) {
    ui.separator();
    ui.label("Рендер");
    let before = *r;
    ui.horizontal_wrapped(|ui| {
        ui.label("Сглаживание");
        for (name, mode) in [
            ("нет", ArchiveAntialiasing::Off),
            ("SMAA", ArchiveAntialiasing::Spatial),
            ("TAA", ArchiveAntialiasing::Temporal),
            ("MetalFX", ArchiveAntialiasing::MetalFx),
        ] {
            ui.selectable_value(&mut r.antialiasing, mode, name);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Тонмаппинг");
        ui.selectable_value(&mut r.tone_mapper, ArchiveToneMapper::Agx, "AgX");
        ui.selectable_value(&mut r.tone_mapper, ArchiveToneMapper::Aces, "ACES");
    });
    let mut dof = r.lens.is_some();
    if ui
        .checkbox(&mut dof, "Глубина резкости и motion blur")
        .changed()
    {
        r.lens = dof.then_some(Lens {
            f_number: 5.6,
            focus_distance_m: 0.,
            shutter_fraction: 0.5,
        });
    }
    if let Some(lens) = r.lens.as_mut() {
        ui.add(
            egui::Slider::new(&mut lens.f_number, 1.4..=22.)
                .logarithmic(true)
                .text("диафрагма f/"),
        );
        ui.add(egui::Slider::new(&mut lens.focus_distance_m, 0.0..=50.).text("фокус м (0 = авто)"));
        ui.add(egui::Slider::new(&mut lens.shutter_fraction, 0.0..=1.).text("затвор"));
    }
    if *r != before {
        *reconfigure = true;
    }
    let mut probes = r.probe_spacing.is_some();
    if ui
        .checkbox(&mut probes, "Пробы отражённого света")
        .changed()
    {
        r.probe_spacing = probes.then_some(6.);
        *rebuild = true;
    }
    if let Some(spacing) = r.probe_spacing.as_mut() {
        let response = ui.add(egui::Slider::new(spacing, 2.0..=12.).text("шаг проб м"));
        // Baking takes a moment, so wait for the drag to end.
        if response.drag_stopped() || (response.changed() && !response.dragged()) {
            *rebuild = true;
        }
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
            *a = crate::original::Ageing {
                damage: 0.,
                colour: 0.,
                relaid: 0.,
                dirt: 0.,
            };
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

/// Named camera viewpoint bookmark with eye coordinates and look direction vector.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TeleportBookmark {
    pub name: &'static str,
    pub position: [f64; 3],
    pub direction: [f32; 3],
}

/// Filters `bookmarks` case-insensitively by substring match on `name`.
#[allow(dead_code)]
pub fn filter_teleport_bookmarks(
    bookmarks: &[TeleportBookmark],
    query: &str,
) -> Vec<TeleportBookmark> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return bookmarks.to_vec();
    }
    bookmarks
        .iter()
        .copied()
        .filter(|b| b.name.to_lowercase().contains(&needle))
        .collect()
}

#[cfg(test)]
mod panel_tests {
    use super::*;

    #[test]
    fn teleport_bookmarks_filter_case_insensitively() {
        let bookmarks = [
            TeleportBookmark {
                name: "Start Square",
                position: [0.0, 1.7, 8.0],
                direction: [0.0, 0.0, 1.0],
            },
            TeleportBookmark {
                name: "Villa Pool",
                position: [2.9, 1.82, 107.4],
                direction: [-0.2, -0.2, 0.96],
            },
        ];
        assert_eq!(filter_teleport_bookmarks(&bookmarks, "").len(), 2);
        let filtered = filter_teleport_bookmarks(&bookmarks, "villa");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Villa Pool");
    }
}
