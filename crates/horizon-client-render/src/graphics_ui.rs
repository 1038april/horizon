//! Horizon's graphics settings screens, Simple, Advanced and HDR: which settings
//! they show, in what groups, with what description and cost, and their
//! input. How they look comes from `graphics_style`.
use crate::{
    graphics::{FarDetail, GraphicsApi, Overlay, Pacing, PresentMode, Preset, Upscaling},
    graphics_style::{Cost, STYLE},
    hud::{HudAtlas, HudBatch},
    settings::{Opt, Settings},
    text::Text,
    ui,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Simple,
    Advanced,
    /// HDR output's own settings, opened from Advanced while HDR is on.
    Hdr,
}

/// Horizon's own settings (`graphics::Graphics`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum H {
    Preset,
    LodDistance,
    FarDetail,
    CaveCulling,
    EntityCulling,
    ChunkUpdates,
    UploadBudget,
    Sky,
    Fog,
    Weather,
    AnimatedTextures,
    PresentMode,
    Pacing,
    Workers,
    Overlay,
    ResolutionScale,
    Antialiasing,
    Upscaling,
    Hdr,
    /// Opens `Level::Hdr`.
    HdrSettings,
    HdrBlackLevel,
    HdrPeak,
    HdrPaperWhite,
    HdrUiBrightness,
    BatteryMode,
    GraphicsApi,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Vanilla(Opt),
    Horizon(H),
}

pub struct Setting {
    pub item: Item,
    pub cost: Cost,
    pub text: &'static str,
}
pub enum Row {
    Header(&'static str),
    Setting(Setting),
}
fn v(opt: Opt, cost: Cost, text: &'static str) -> Row {
    Row::Setting(Setting {
        item: Item::Vanilla(opt),
        cost,
        text,
    })
}
fn h(opt: H, cost: Cost, text: &'static str) -> Row {
    Row::Setting(Setting {
        item: Item::Horizon(opt),
        cost,
        text,
    })
}

/// The settings of a level, grouped under headers (Simple has no headers).
pub fn rows(level: Level) -> Vec<Row> {
    use Cost::*;
    let render = v(
        Opt::RenderDistance,
        High,
        "Full-detail terrain range. LOD Distance can extend local worlds farther.",
    );
    let lod = h(
        H::LodDistance,
        Medium,
        "Coarse distant terrain in local worlds. Integrated GPUs stop at 128 chunks.",
    );
    // Suggested, never switched on, where far terrain costs the GPU most.
    let far = h(
        H::FarDetail,
        Medium,
        if crate::graphics::reduced_far_detail_suggested() {
            "Reduced draws far terrain coarser: faster on this integrated GPU."
        } else {
            "Reduced draws far terrain coarser, for GPUs it slows down."
        },
    );
    let fov = v(
        Opt::Fov,
        Low,
        "Field of view. Wider shows more terrain at once.",
    );
    let fps = v(
        Opt::MaxFps,
        None,
        "Caps frames per second; a cap saves power and heat.",
    );
    let vsync = v(
        Opt::Vsync,
        None,
        "Waits for the display's refresh: no tearing, a little latency.",
    );
    let gui = v(Opt::GuiScale, None, "Size of menus and the HUD.");
    let preset = h(
        H::Preset,
        None,
        "Sets the options below together. Changing one makes it Custom.",
    );
    let mut rows = match level {
        Level::Simple => vec![preset, render, lod, far, fov, fps, vsync, gui],
        Level::Hdr => vec![
            Row::Header("HDR"),
            h(
                H::Hdr,
                None,
                "Lava, fire and sunlight shine brighter than white. Needs an HDR display.",
            ),
            h(
                H::HdrBlackLevel,
                None,
                "Your display's darkest black. 0 for OLED; LCDs glow a little.",
            ),
            h(
                H::HdrPeak,
                None,
                "How bright highlights get. Auto: the peak your system reports.",
            ),
            h(
                H::HdrPaperWhite,
                None,
                "How bright white paper and menus look. Auto: your desktop's white.",
            ),
            h(
                H::HdrUiBrightness,
                None,
                "How bright the HUD and menus are, against paper white.",
            ),
        ],
        Level::Advanced => vec![
            Row::Header("Rendering"),
            render,
            lod,
            far,
            v(
                Opt::SimulationDistance,
                Medium,
                "How far mobs, crops and machines keep working. CPU time.",
            ),
            v(
                Opt::EntityDistance,
                Medium,
                "How far away mobs and items are drawn, as a share of normal.",
            ),
            h(
                H::CaveCulling,
                Saves,
                "Skips terrain hidden behind closed sections. Off costs a lot.",
            ),
            h(
                H::EntityCulling,
                None,
                "Hides models behind opaque blocks. Entities update. Off with custom packs.",
            ),
            h(
                H::ChunkUpdates,
                Low,
                "Meshed sections uploaded per frame. Lower is smoother while loading.",
            ),
            Row::Header("Quality"),
            v(
                Opt::SmoothLighting,
                Low,
                "Soft shadows in corners. Costs chunk building time only.",
            ),
            v(
                Opt::Mipmaps,
                None,
                "Smaller copies of block textures for distant terrain: less shimmer.",
            ),
            v(
                Opt::BiomeBlend,
                Low,
                "Blends grass and water colours across biomes when building chunks.",
            ),
            v(
                Opt::Clouds,
                Low,
                "Cloud layer: off, flat (fast) or 3D (fancy).",
            ),
            v(Opt::CloudRange, Low, "How far clouds reach, in chunks."),
            h(H::Sky, Low, "Draws the sky, sun, moon and stars."),
            h(
                H::Fog,
                None,
                "Distance fog. Off shows the edge of the loaded world.",
            ),
            h(H::Weather, High, "Draws rain and snow."),
            v(
                Opt::WeatherRadius,
                Medium,
                "How far rain and snow fall around you.",
            ),
            v(Opt::Particles, Medium, "How many particles effects spawn."),
            v(
                Opt::EntityShadows,
                Low,
                "Round shadows under mobs and items.",
            ),
            v(
                Opt::CutoutLeaves,
                Medium,
                "Leaves with see-through gaps. Off draws them solid.",
            ),
            v(
                Opt::ImprovedTransparency,
                Medium,
                "Sorts water and glass more exactly behind each other.",
            ),
            v(
                Opt::ChunkBuilder,
                Low,
                "Which chunk rebuilds wait for the frame: none, nearby or yours.",
            ),
            h(
                H::AnimatedTextures,
                Low,
                "Animates water, lava, fire and other moving textures.",
            ),
            Row::Header("Display"),
            fov,
            fps,
            vsync,
            h(
                H::ResolutionScale,
                High,
                "World pixels per screen pixel. Below 100% is faster on the GPU.",
            ),
            h(
                H::Antialiasing,
                Medium,
                "Smooths jagged edges. FXAA: softer. TAA: smoothest, moving things may trail.",
            ),
            h(
                H::Upscaling,
                High,
                "Renders the world smaller and rebuilds it with TAA. Faster, softer.",
            ),
            h(
                H::HdrSettings,
                None,
                "HDR output on an HDR display: lava, fire and sunlight shine brighter.",
            ),
            h(
                H::PresentMode,
                None,
                "How frames reach the screen while VSync is off.",
            ),
            h(
                H::Pacing,
                None,
                "Efficient sleeps until the next frame; Precise spins, like vanilla.",
            ),
            v(Opt::Fullscreen, None, "Fills the screen (borderless)."),
            gui,
            v(Opt::Gamma, None, "Brightens dark places."),
            v(
                Opt::InactivityFps,
                None,
                "Lowers the frame rate when minimised, or idle too.",
            ),
            Row::Header("Preferences"),
            v(Opt::ViewBobbing, None, "Sways the view while walking."),
            v(Opt::ChunkFade, None, "Fades newly loaded chunks in."),
            v(Opt::Vignette, None, "Darkens the screen's edges."),
            v(Opt::Autosave, None, "Shows when the world saves."),
            v(Opt::AttackIndicator, None, "Shows attack cooldown."),
            Row::Header("Performance"),
            h(
                H::BatteryMode,
                None,
                "60 FPS cap (30 in menus or unfocused), half the workers: less power.",
            ),
            h(
                H::Workers,
                None,
                "Threads that generate, light and mesh chunks. Next start.",
            ),
            h(
                H::GraphicsApi,
                None,
                "Vulkan, then DirectX 12 (Windows) or OpenGL (Linux) where it fails. Next start.",
            ),
            h(
                H::UploadBudget,
                Low,
                "Mesh data sent to the GPU per frame. Lower is smoother while loading.",
            ),
            Row::Header("Overlay"),
            h(
                H::Overlay,
                None,
                "Frame rate and frame times over the game, with a graph.",
            ),
        ],
    };
    // Only Windows and Linux have more than one graphics API.
    if cfg!(target_os = "macos") {
        rows.retain(
            |row| !matches!(row, Row::Setting(s) if s.item == Item::Horizon(H::GraphicsApi)),
        );
    }
    rows
}

const UPLOAD_STEPS: [u32; 8] = [0, 256, 512, 1024, 2048, 4096, 8192, 16384];

/// Horizon's graphics screens supplement the vanilla options on Video Settings.
/// Upscaling is only there while TAA is, which does it.
fn extra_rows(level: Level, settings: &Settings) -> Vec<Row> {
    let mut rows = rows(level);
    rows.retain(|row| !matches!(row, Row::Setting(s) if matches!(s.item, Item::Vanilla(_))));
    if !settings.horizon.taa {
        rows.retain(|row| !matches!(row, Row::Setting(s) if s.item == Item::Horizon(H::Upscaling)));
    }
    // Drop headings whose settings were all on the parent screens.
    let mut has_setting = false;
    rows.reverse();
    rows.retain(|row| match row {
        Row::Setting(_) => {
            has_setting = true;
            true
        }
        Row::Header(_) => std::mem::take(&mut has_setting),
    });
    rows.reverse();
    rows
}

fn on_off(on: bool) -> &'static str {
    if on { "On" } else { "Off" }
}
fn labelled(caption: &str, value: impl std::fmt::Display) -> Text {
    Text::literal(format!("{caption}: {value}"))
}

impl H {
    fn slider(self, s: &Settings) -> Option<f32> {
        let g = &s.horizon;
        match self {
            H::ChunkUpdates => Some(g.chunk_updates as f32 / 64.0),
            H::UploadBudget => {
                let i = UPLOAD_STEPS
                    .iter()
                    .position(|&b| b >= g.upload_budget_kib)
                    .unwrap_or(UPLOAD_STEPS.len() - 1);
                Some(i as f32 / (UPLOAD_STEPS.len() - 1) as f32)
            }
            H::Workers => Some(g.workers as f32 / max_workers() as f32),
            H::ResolutionScale => Some((g.resolution_scale as f32 - 50.0) / 150.0),
            // Auto, then 80..480 by 10.
            H::HdrPaperWhite => Some(match g.hdr_paper_white {
                0 => 0.0,
                n => (n as f32 - 70.0) / 410.0,
            }),
            // Auto, then 200..2000 by 50.
            H::HdrPeak => Some(match g.hdr_peak {
                0 => 0.0,
                n => (n as f32 - 150.0) / 1850.0,
            }),
            H::HdrBlackLevel => Some(g.hdr_black_level as f32 / 100.0),
            H::HdrUiBrightness => Some((g.hdr_ui_brightness as f32 - 25.0) / 75.0),
            _ => None,
        }
    }
    fn label(self, s: &Settings) -> Text {
        let g = &s.horizon;
        match self {
            H::Preset => labelled("Preset", g.preset.label()),
            H::LodDistance => match g.lod_distance {
                0 => labelled("LOD Distance", "Off"),
                n => labelled("LOD Distance", format!("{n} chunks")),
            },
            H::FarDetail => labelled("Far Detail", g.far_detail.label()),
            H::CaveCulling => labelled("Cave Culling", on_off(g.cave_culling)),
            H::EntityCulling => labelled("Entity Culling", on_off(g.entity_culling)),
            H::ChunkUpdates => match g.chunk_updates {
                0 => labelled("Chunk Updates", "Unlimited"),
                n => labelled("Chunk Updates", format!("{n} per frame")),
            },
            H::UploadBudget => match g.upload_budget_kib {
                0 => labelled("Upload Budget", "Unlimited"),
                k if k >= 1024 => labelled("Upload Budget", format!("{} MiB/frame", k / 1024)),
                k => labelled("Upload Budget", format!("{k} KiB/frame")),
            },
            H::Sky => labelled("Sky", on_off(g.sky)),
            H::Fog => labelled("Fog", on_off(g.fog)),
            H::Weather => labelled("Weather", on_off(g.weather)),
            H::AnimatedTextures => labelled("Animated Textures", on_off(g.animated_textures)),
            H::PresentMode => labelled("Present Mode", g.present_mode.label()),
            H::Pacing => labelled(
                "Frame Pacing",
                match g.pacing {
                    Pacing::Efficient => "Efficient",
                    Pacing::Precise => "Precise",
                },
            ),
            H::Workers => match g.workers {
                0 => labelled("Worker Threads", format!("Auto ({})", g.worker_threads())),
                n => labelled("Worker Threads", n),
            },
            H::ResolutionScale => labelled("Resolution Scale", format!("{}%", g.resolution_scale)),
            H::Antialiasing => labelled(
                "Anti-aliasing",
                match (g.fxaa, g.taa) {
                    (_, true) => "TAA",
                    (true, false) => "FXAA",
                    (false, false) => "Off",
                },
            ),
            H::Upscaling => labelled("Upscaling", g.upscaling.label()),
            H::Hdr => labelled("HDR", on_off(g.hdr)),
            H::HdrSettings => Text::literal("HDR Settings..."),
            H::HdrPaperWhite => match g.hdr_paper_white {
                0 => labelled(
                    "Paper White",
                    format!("Auto ({} nits)", g.paper_white_nits()),
                ),
                n => labelled("Paper White", format!("{n} nits")),
            },
            H::HdrPeak => match g.hdr_peak {
                0 => labelled("Peak Brightness", format!("Auto ({} nits)", g.peak_nits())),
                n => labelled("Peak Brightness", format!("{n} nits")),
            },
            H::HdrBlackLevel => labelled(
                "Black Level",
                format!("{:.2} nits", g.hdr_black_level as f32 / 100.0),
            ),
            H::HdrUiBrightness => labelled("UI Brightness", format!("{}%", g.hdr_ui_brightness)),
            H::BatteryMode => labelled("Battery Mode", on_off(g.battery)),
            H::GraphicsApi => labelled("Graphics API", g.api.label()),
            H::Overlay => labelled(
                "Performance Overlay",
                match g.overlay {
                    Overlay::Off => "Off",
                    Overlay::Fps => "FPS",
                    Overlay::Graph => "FPS and Graph",
                },
            ),
        }
    }
    fn set_slider(self, s: &mut Settings, v: f32) {
        let g = &mut s.horizon;
        match self {
            H::ChunkUpdates => g.chunk_updates = (v * 64.0).round() as u32,
            H::UploadBudget => {
                g.upload_budget_kib =
                    UPLOAD_STEPS[(v * (UPLOAD_STEPS.len() - 1) as f32).round() as usize]
            }
            H::Workers => g.workers = (v * max_workers() as f32).round() as u32,
            H::ResolutionScale => g.resolution_scale = (50.0 + (v * 30.0).round() * 5.0) as u32,
            H::HdrPaperWhite => {
                g.hdr_paper_white = match (v * 41.0).round() as u32 {
                    0 => 0,
                    step => 70 + step * 10,
                }
            }
            H::HdrPeak => {
                g.hdr_peak = match (v * 37.0).round() as u32 {
                    0 => 0,
                    step => 150 + step * 50,
                }
            }
            H::HdrBlackLevel => g.hdr_black_level = (v * 100.0).round() as u32,
            H::HdrUiBrightness => g.hdr_ui_brightness = (25.0 + (v * 15.0).round() * 5.0) as u32,
            _ => {}
        }
    }
    fn cycle(self, s: &mut Settings, back: bool) {
        fn step<T: Copy + PartialEq>(all: &[T], now: T, back: bool) -> T {
            let i = all.iter().position(|x| *x == now).unwrap_or(0);
            let n = all.len();
            all[if back { (i + n - 1) % n } else { (i + 1) % n }]
        }
        if self == H::Preset {
            let preset = step(&Preset::ALL, s.horizon.preset, back);
            crate::graphics::apply_preset(preset, s);
            s.horizon.preset = preset;
            return;
        }
        let g = &mut s.horizon;
        match self {
            H::LodDistance => {
                g.lod_distance = step(&[0, 64, 128, 256, 512, 1024], g.lod_distance, back)
            }
            H::FarDetail => {
                g.far_detail = step(&[FarDetail::Full, FarDetail::Reduced], g.far_detail, back)
            }
            H::CaveCulling => g.cave_culling = !g.cave_culling,
            H::EntityCulling => g.entity_culling = !g.entity_culling,
            H::Sky => g.sky = !g.sky,
            H::Fog => g.fog = !g.fog,
            H::Weather => g.weather = !g.weather,
            H::AnimatedTextures => g.animated_textures = !g.animated_textures,
            H::PresentMode => g.present_mode = step(&PresentMode::ALL, g.present_mode, back),
            // Off, FXAA, TAA.
            H::Antialiasing => {
                let i = if g.taa { 2 } else { usize::from(g.fxaa) };
                let i = (i + if back { 2 } else { 1 }) % 3;
                (g.fxaa, g.taa) = (i == 1, i == 2);
                if !g.taa {
                    g.upscaling = Upscaling::Native;
                }
            }
            // Upscaling is TAA's: picking a ratio turns it on.
            H::Upscaling => {
                g.upscaling = step(&Upscaling::ALL, g.upscaling, back);
                if g.upscaling != Upscaling::Native {
                    (g.fxaa, g.taa) = (false, true);
                }
            }
            H::Hdr => g.hdr = !g.hdr,
            H::BatteryMode => g.battery = !g.battery,
            H::GraphicsApi => g.api = step(GraphicsApi::choices(), g.api, back),
            H::Pacing => {
                g.pacing = match g.pacing {
                    Pacing::Efficient => Pacing::Precise,
                    Pacing::Precise => Pacing::Efficient,
                }
            }
            H::Overlay => {
                g.overlay = step(
                    &[Overlay::Off, Overlay::Fps, Overlay::Graph],
                    g.overlay,
                    back,
                )
            }
            _ => {}
        }
    }
}
fn max_workers() -> u32 {
    std::thread::available_parallelism().map_or(8, |n| n.get() as u32)
}

impl Item {
    /// False for a setting whose system is not in Horizon yet (`Opt::available`).
    fn available(self) -> bool {
        !matches!(self, Item::Vanilla(o) if !o.available())
    }
    /// Why a control can't be used now, when it can't: HDR's screen opens only where
    /// the display offers HDR output, whatever `horizon-graphics.txt` says (a copied game
    /// folder, a display swapped for an SDR one: the renderer leaves HDR off there, and
    /// the saved values wait for an HDR display), and its sliders do something only
    /// while it is on.
    fn blocked(self, s: &Settings) -> Option<&'static str> {
        if !self.available() {
            return Some("This feature is not yet available in Horizon.");
        }
        match self {
            Item::Horizon(H::HdrSettings | H::Hdr) if !crate::graphics::hdr_offered() => {
                Some("Your display or system doesn't offer HDR output.")
            }
            Item::Horizon(
                H::HdrBlackLevel | H::HdrPeak | H::HdrPaperWhite | H::HdrUiBrightness,
            ) if !s.horizon.hdr => Some("Turn HDR on to use it."),
            _ => None,
        }
    }
    /// On/off settings and their value.
    pub fn switch(self, s: &Settings) -> Option<bool> {
        let g = &s.horizon;
        Some(match self {
            Item::Horizon(H::CaveCulling) => g.cave_culling,
            Item::Horizon(H::EntityCulling) => g.entity_culling,
            Item::Horizon(H::Sky) => g.sky,
            Item::Horizon(H::Fog) => g.fog,
            Item::Horizon(H::Weather) => g.weather,
            Item::Horizon(H::AnimatedTextures) => g.animated_textures,
            Item::Horizon(H::Hdr) => g.hdr,
            Item::Horizon(H::BatteryMode) => g.battery,
            Item::Vanilla(Opt::ViewBobbing) => s.view_bobbing,
            Item::Vanilla(Opt::Vsync) => s.vsync,
            Item::Vanilla(Opt::Fullscreen) => s.fullscreen,
            Item::Vanilla(Opt::SmoothLighting) => s.smooth_lighting,
            Item::Vanilla(Opt::EntityShadows) => s.entity_shadows,
            Item::Vanilla(Opt::Vignette) => s.vignette,
            Item::Vanilla(Opt::Autosave) => s.autosave_indicator,
            Item::Vanilla(Opt::CutoutLeaves) => s.cutout_leaves,
            Item::Vanilla(Opt::ImprovedTransparency) => s.improved_transparency,
            _ => return None,
        })
    }
    pub fn slider(self, s: &Settings) -> Option<f32> {
        match self {
            Item::Vanilla(o) => o.slider_position(s),
            Item::Horizon(h) => h.slider(s),
        }
    }
    pub fn label(self, s: &Settings) -> Text {
        match self {
            Item::Vanilla(Opt::RenderDistance) if s.horizon.lod_distance > 0 => labelled(
                "Full Detail Distance",
                format!("{} chunks", s.render_distance),
            ),
            Item::Vanilla(o) => o.label(s),
            Item::Horizon(h) => h.label(s),
        }
    }
    pub fn set_slider(self, s: &mut Settings, v: f32) {
        if !self.available() {
            return;
        }
        match self {
            Item::Vanilla(o) => o.set_slider(s, v),
            Item::Horizon(h) => h.set_slider(s, v),
        }
        self.touched(s);
    }
    pub fn cycle(self, s: &mut Settings, back: bool, framebuffer: [i32; 2]) {
        if !self.available() {
            return;
        }
        match self {
            Item::Vanilla(o) => o.cycle(s, back, framebuffer),
            Item::Horizon(h) => h.cycle(s, back),
        }
        self.touched(s);
    }
    /// A preset no longer describes the settings once one of them changes.
    fn touched(self, s: &mut Settings) {
        let presetless = matches!(
            self,
            Item::Horizon(
                H::Preset
                    | H::Overlay
                    | H::Pacing
                    | H::PresentMode
                    | H::Workers
                    | H::BatteryMode
                    | H::GraphicsApi
            ) | Item::Vanilla(
                Opt::GuiScale
                    | Opt::Fullscreen
                    | Opt::Vsync
                    | Opt::MaxFps
                    | Opt::Fov
                    | Opt::Gamma
                    | Opt::InactivityFps
                    | Opt::ViewBobbing
                    | Opt::ChunkFade
                    | Opt::Vignette
                    | Opt::Autosave
                    | Opt::AttackIndicator
            )
        );
        if !presetless {
            s.horizon.preset = Preset::Custom;
        }
    }
    pub fn step_slider(self, s: &mut Settings, forward: bool, width: f32) {
        if !self.available() || self.blocked(s).is_some() {
            return;
        }
        match self {
            Item::Vanilla(opt) => match opt {
                Opt::Fov => s.fov = (s.fov + if forward { 1.0 } else { -1.0 }).clamp(30.0, 110.0),
                Opt::RenderDistance => {
                    s.render_distance = (s.render_distance + if forward { 1 } else { -1 })
                        .clamp(2, crate::options_file::MAX_RENDER_DISTANCE);
                }
                Opt::SimulationDistance => {
                    s.simulation_distance =
                        (s.simulation_distance + if forward { 1 } else { -1 }).clamp(5, 32);
                }
                Opt::MaxFps => {
                    s.max_fps =
                        (s.max_fps as i32 + if forward { 10 } else { -10 }).clamp(10, 260) as u32;
                }
                Opt::CloudRange => {
                    s.cloud_range = (s.cloud_range + if forward { 1 } else { -1 }).clamp(2, 128);
                }
                Opt::EntityDistance => {
                    s.entity_distance =
                        (s.entity_distance + if forward { 0.25 } else { -0.25 }).clamp(0.5, 5.0);
                }
                Opt::BiomeBlend => {
                    s.biome_blend = (s.biome_blend + if forward { 1 } else { -1 }).clamp(0, 7);
                }
                Opt::WeatherRadius => {
                    s.weather_radius =
                        (s.weather_radius + if forward { 1 } else { -1 }).clamp(3, 10);
                }
                Opt::Mipmaps => {
                    s.mipmap_levels = (s.mipmap_levels + if forward { 1 } else { -1 }).clamp(0, 4);
                }
                _ => {
                    let delta = 1.0 / (width - 8.0).max(1.0);
                    let step = if forward { delta } else { -delta };
                    if let Some(pos) = opt.slider_position(s) {
                        opt.set_slider(s, (pos + step).clamp(0.0, 1.0));
                    }
                }
            },
            Item::Horizon(h) => {
                let g = &mut s.horizon;
                match h {
                    H::ChunkUpdates => {
                        g.chunk_updates = (g.chunk_updates as i32 + if forward { 1 } else { -1 })
                            .clamp(0, 64) as u32;
                    }
                    H::UploadBudget => {
                        let i = UPLOAD_STEPS
                            .iter()
                            .position(|&b| b >= g.upload_budget_kib)
                            .unwrap_or(UPLOAD_STEPS.len() - 1);
                        let next = if forward {
                            (i + 1).min(UPLOAD_STEPS.len() - 1)
                        } else {
                            i.saturating_sub(1)
                        };
                        g.upload_budget_kib = UPLOAD_STEPS[next];
                    }
                    H::Workers => {
                        g.workers = (g.workers as i32 + if forward { 1 } else { -1 })
                            .clamp(0, max_workers() as i32)
                            as u32;
                    }
                    H::ResolutionScale => {
                        g.resolution_scale = (g.resolution_scale as i32
                            + if forward { 5 } else { -5 })
                        .clamp(50, 200) as u32;
                    }
                    H::HdrPaperWhite => {
                        let cur = g.hdr_paper_white as i32;
                        let next = if forward {
                            if cur == 0 { 80 } else { (cur + 10).min(480) }
                        } else {
                            if cur <= 80 { 0 } else { cur - 10 }
                        };
                        g.hdr_paper_white = next as u32;
                    }
                    H::HdrPeak => {
                        let cur = g.hdr_peak as i32;
                        let next = if forward {
                            if cur == 0 { 200 } else { (cur + 50).min(2000) }
                        } else {
                            if cur <= 200 { 0 } else { cur - 50 }
                        };
                        g.hdr_peak = next as u32;
                    }
                    H::HdrBlackLevel => {
                        g.hdr_black_level = (g.hdr_black_level as i32
                            + if forward { 1 } else { -1 })
                        .clamp(0, 100) as u32;
                    }
                    H::HdrUiBrightness => {
                        g.hdr_ui_brightness = (g.hdr_ui_brightness as i32
                            + if forward { 5 } else { -5 })
                        .clamp(25, 100) as u32;
                    }
                    _ => {}
                }
            }
        }
        self.touched(s);
    }
}

/// What the options screens must do after an interaction here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    ButtonClick,
    Changed,
    SliderDrag,
    OpenAdvanced,
    OpenSimple,
    OpenHdr,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Item(Item),
    Button(Action),
}
struct Widget {
    x: f32,
    y: f32,
    w: f32,
    target: Target,
    /// In the scrolling list (clipped to it).
    list: bool,
}

#[derive(Default)]
pub struct GraphicsUi {
    /// The list's scroll amount and its held scrollbar (`AbstractScrollArea`).
    scroll: ui::Scroll,
    dragging: Option<Item>,
    pub focus: Option<Target>,
    pub can_change_value: bool,
}

impl GraphicsUi {
    pub fn reset(&mut self) {
        self.scroll = ui::Scroll::default();
        self.dragging = None;
        self.focus = None;
        self.can_change_value = false;
    }
    /// The scrollbar's column: right of the descriptions.
    fn scrollbar_x(w: f32) -> f32 {
        STYLE.row_x(w) + STYLE.control_width + STYLE.gap + STYLE.text_width(w) + 6.0
    }
    fn bounds(h: f32) -> (f32, f32) {
        (STYLE.band, h - STYLE.band)
    }
    fn content_height(level: Level, s: &Settings) -> f32 {
        extra_rows(level, s).iter().fold(4.0, |height, row| {
            height
                + match row {
                    Row::Header(_) => STYLE.header_height,
                    Row::Setting(_) => STYLE.row_height,
                }
        })
    }
    /// Widgets and the list's content height, using a bounded offset.
    fn layout(
        &mut self,
        level: Level,
        s: &Settings,
        w: f32,
        h: f32,
    ) -> (Vec<Widget>, Vec<(f32, &'static str)>, f32) {
        let content = Self::content_height(level, s);
        self.clamp_scroll(h, content);
        let (top, _) = Self::bounds(h);
        let x = STYLE.row_x(w);
        let mut y = top + 4.0 - self.scroll.amount;
        let mut widgets = Vec::new();
        let mut headers = Vec::new();
        for row in extra_rows(level, s) {
            match row {
                Row::Header(name) => {
                    headers.push((y, name));
                    y += STYLE.header_height;
                }
                Row::Setting(s) => {
                    widgets.push(Widget {
                        x,
                        y,
                        w: STYLE.control_width,
                        target: Target::Item(s.item),
                        list: true,
                    });
                    y += STYLE.row_height;
                }
            }
        }
        let cx = (w / 2.0).floor();
        let by = h - 27.0;
        let other = match level {
            Level::Simple => Action::OpenAdvanced,
            Level::Advanced => Action::OpenSimple,
            // One Done, back to Advanced.
            Level::Hdr => {
                widgets.push(Widget {
                    x: cx - 100.0,
                    y: by,
                    w: 200.0,
                    target: Target::Button(Action::Done),
                    list: false,
                });
                return (widgets, headers, content);
            }
        };
        widgets.push(Widget {
            x: cx - 155.0,
            y: by,
            w: 150.0,
            target: Target::Button(other),
            list: false,
        });
        widgets.push(Widget {
            x: cx + 5.0,
            y: by,
            w: 150.0,
            target: Target::Button(Action::Done),
            list: false,
        });
        (widgets, headers, content)
    }
    fn clamp_scroll(&mut self, h: f32, content: f32) {
        let (top, bottom) = Self::bounds(h);
        self.scroll.clamp(top, bottom, content);
    }
    pub fn draw(
        &mut self,
        level: Level,
        s: &Settings,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        in_world: bool,
    ) {
        let (w, h) = (batch.width, batch.height);
        let (widgets, headers, content) = self.layout(level, s, w, h);
        let (top, bottom) = Self::bounds(h);
        STYLE.background(batch, atlas, top, bottom, in_world);
        let shown = |y: f32, height: f32| y >= top && y + height <= bottom;
        let x = STYLE.row_x(w);
        for (y, name) in headers {
            if shown(y, STYLE.header_height) {
                STYLE.header(batch, atlas, &Text::literal(name), x, y);
            }
        }
        let descriptions: Vec<(Cost, &'static str)> = extra_rows(level, s)
            .into_iter()
            .filter_map(|r| match r {
                Row::Setting(s) => Some((s.cost, s.text)),
                Row::Header(_) => None,
            })
            .collect();
        let mut index = 0;
        let mut tooltip = None;
        for wd in &widgets {
            let is_focused = self.focus == Some(wd.target);
            let hover = (ui::hovered(cursor, wd.x, wd.y, wd.w, 20.0)
                && (!wd.list || (cursor.1 >= top && cursor.1 < bottom)))
                || is_focused;
            match wd.target {
                Target::Item(item) => {
                    let (cost, text) = descriptions[index];
                    index += 1;
                    if !shown(wd.y, 20.0) {
                        continue;
                    }
                    let label = item.label(s);
                    let blocked = item.blocked(s);
                    let live = blocked.is_none();
                    if hover && let Some(why) = blocked {
                        tooltip = Some(atlas.font.split(&Text::literal(why), 170.0));
                    }
                    match item.slider(s) {
                        Some(value) => STYLE.slider_ext(
                            batch,
                            atlas,
                            (wd.x, wd.y, wd.w),
                            &label,
                            value,
                            hover || self.dragging == Some(item),
                            is_focused,
                            self.can_change_value && is_focused,
                            live,
                            self.dragging == Some(item),
                        ),
                        None => STYLE.button(
                            batch,
                            atlas,
                            (wd.x, wd.y, wd.w),
                            &label,
                            hover && live,
                            live,
                        ),
                    }
                    STYLE.description(
                        batch,
                        atlas,
                        wd.x + wd.w + STYLE.gap,
                        wd.y + 1.0,
                        cost,
                        text,
                    );
                }
                Target::Button(action) => {
                    let label = match action {
                        Action::OpenAdvanced => "Advanced...",
                        Action::OpenSimple => "Simple...",
                        _ => "Done",
                    };
                    STYLE.button(
                        batch,
                        atlas,
                        (wd.x, wd.y, wd.w),
                        &Text::literal(label),
                        hover,
                        true,
                    );
                }
            }
        }
        STYLE.frame(batch, atlas, top, bottom, in_world);
        ui::scrollbar(
            batch,
            atlas,
            Self::scrollbar_x(w),
            top,
            bottom,
            content,
            self.scroll.amount,
        );
        let title = match level {
            Level::Simple => "Graphics",
            Level::Advanced => "Graphics: Advanced",
            Level::Hdr => "Graphics: HDR",
        };
        STYLE.title(batch, atlas, &Text::literal(title));
        if let Some(lines) = tooltip {
            ui::tooltip(batch, atlas, &lines, cursor);
        }
    }
    /// A press (or a drag with `drag`) at `(x, y)`.
    pub fn click(
        &mut self,
        level: Level,
        s: &mut Settings,
        (x, y): (f32, f32),
        (w, h): (f32, f32),
        framebuffer: [i32; 2],
        drag: bool,
        right: bool,
    ) -> Vec<Action> {
        let (widgets, _, content) = self.layout(level, s, w, h);
        let (top, bottom) = Self::bounds(h);
        if drag {
            // AbstractScrollArea.mouseDragged: the held scrollbar follows the pointer.
            if self.scroll.held() {
                self.scroll.drag(y, top, bottom, content);
                return Vec::new();
            }
            if let Some(item) = self.dragging
                && let Some(wd) = widgets
                    .iter()
                    .find(|wd| matches!(wd.target, Target::Item(i) if i == item))
            {
                item.set_slider(s, ui::slider_value(wd.x, wd.w, x));
                return vec![Action::SliderDrag];
            }
            return Vec::new();
        }
        if right {
            return Vec::new();
        }
        // AbstractScrollArea.updateScrolling: a press on the scrollbar holds it, silently.
        if self
            .scroll
            .press((x, y), Self::scrollbar_x(w), top, bottom, content)
        {
            return Vec::new();
        }
        for wd in widgets {
            if !ui::hovered((x, y), wd.x, wd.y, wd.w, 20.0) {
                continue;
            }
            if wd.list && (y < top || y >= bottom || wd.y < top || wd.y + 20.0 > bottom) {
                continue;
            }
            self.focus = Some(wd.target);
            return match wd.target {
                Target::Button(action) => {
                    self.can_change_value = false;
                    vec![Action::ButtonClick, action]
                }
                // A disabled control does nothing: no sound, no preset change.
                Target::Item(item) if item.blocked(s).is_some() => Vec::new(),
                Target::Item(Item::Horizon(H::HdrSettings)) => {
                    self.can_change_value = false;
                    vec![Action::ButtonClick, Action::OpenHdr]
                }
                Target::Item(item) => {
                    if item.slider(s).is_some() {
                        self.can_change_value = true;
                        self.dragging = Some(item);
                        item.set_slider(s, ui::slider_value(wd.x, wd.w, x));
                        vec![Action::SliderDrag]
                    } else {
                        self.can_change_value = false;
                        item.cycle(s, right, framebuffer);
                        vec![Action::ButtonClick, Action::Changed]
                    }
                }
            };
        }
        Vec::new()
    }
    /// Lets go of a slider (true: it plays its click) or the scrollbar.
    pub fn release(&mut self) -> bool {
        self.scroll.release();
        self.dragging.take().is_some()
    }
    pub fn scroll(&mut self, level: Level, s: &Settings, h: f32, amount: f32) {
        let content = Self::content_height(level, s);
        let (top, bottom) = Self::bounds(h);
        self.scroll.clamp(top, bottom, content);
        self.scroll.wheel(amount, 18.0, top, bottom, content);
    }

    /// Mouse wheel over a control steps it (slider) or cycles it (button).
    pub fn wheel(
        &mut self,
        level: Level,
        s: &mut Settings,
        cursor: (f32, f32),
        (w, h): (f32, f32),
        framebuffer: [i32; 2],
        amount: f32,
    ) -> Vec<Action> {
        let (widgets, _, _) = self.layout(level, s, w, h);
        let (top, bottom) = Self::bounds(h);
        let hovered = widgets.iter().find(|wd| {
            ui::hovered(cursor, wd.x, wd.y, wd.w, 20.0)
                && (!wd.list
                    || (cursor.1 >= top
                        && cursor.1 < bottom
                        && wd.y >= top
                        && wd.y + 20.0 <= bottom))
        });
        if let Some(wd) = hovered
            && let Target::Item(item) = wd.target
        {
            if item.blocked(s).is_some() {
                return Vec::new();
            }
            if item.slider(s).is_some() {
                if amount != 0.0 {
                    item.step_slider(s, amount > 0.0, wd.w);
                    return vec![Action::SliderDrag];
                }
            } else if !matches!(item, Item::Horizon(H::HdrSettings)) && amount != 0.0 {
                item.cycle(s, amount > 0.0, framebuffer);
                return vec![Action::ButtonClick, Action::Changed];
            }
        }
        self.scroll(level, s, h, amount);
        Vec::new()
    }

    /// Keyboard navigation and value adjustments.
    pub fn key_modifiers(
        &mut self,
        level: Level,
        s: &mut Settings,
        key: winit::keyboard::KeyCode,
        shift: bool,
        (w, h): (f32, f32),
        framebuffer: [i32; 2],
    ) -> (Vec<Action>, bool) {
        use winit::keyboard::KeyCode as K;
        let (widgets, _, content) = self.layout(level, s, w, h);
        let (top, bottom) = Self::bounds(h);
        let focusables: Vec<(Target, [f32; 4], bool)> = widgets
            .iter()
            .filter_map(|wd| match wd.target {
                Target::Item(item) if item.available() && item.blocked(s).is_none() => {
                    Some((wd.target, [wd.x, wd.y, wd.w, 20.0], wd.list))
                }
                Target::Button(_) => Some((wd.target, [wd.x, wd.y, wd.w, 20.0], wd.list)),
                _ => None,
            })
            .collect();

        if focusables.is_empty() {
            return (Vec::new(), false);
        }

        // 1. Tab / Shift+Tab navigation
        if key == K::Tab {
            let cur = self
                .focus
                .and_then(|f| focusables.iter().position(|&(t, ..)| t == f));
            let next_idx = match cur {
                Some(i) => {
                    if shift {
                        if i == 0 { focusables.len() - 1 } else { i - 1 }
                    } else {
                        (i + 1) % focusables.len()
                    }
                }
                None => {
                    if shift {
                        focusables.len() - 1
                    } else {
                        0
                    }
                }
            };
            let (target, [_, wy, _, wh], is_list) = focusables[next_idx];
            self.focus = Some(target);
            let is_slider = matches!(target, Target::Item(item) if item.slider(s).is_some());
            self.can_change_value = is_slider;
            if is_list {
                if wy < top + 4.0 {
                    self.scroll.amount -= top + 4.0 - wy;
                } else if wy + wh > bottom - 4.0 {
                    self.scroll.amount += wy + wh - (bottom - 4.0);
                }
                self.clamp_scroll(h, content);
            }
            return (Vec::new(), true);
        }

        // 2. Left / Right Arrow keys
        if matches!(key, K::ArrowLeft | K::ArrowRight) {
            let forward = key == K::ArrowRight;
            if let Some(Target::Item(item)) = self.focus
                && item.slider(s).is_some()
                && self.can_change_value
            {
                let width = widgets
                    .iter()
                    .find(|wd| wd.target == Target::Item(item))
                    .map_or(STYLE.control_width, |wd| wd.w);
                item.step_slider(s, forward, width);
                return (vec![Action::SliderDrag], true);
            }
            if let Some(cur_target) = self.focus
                && let Some(cur_pos) = focusables.iter().position(|&(t, ..)| t == cur_target)
            {
                let from = focusables[cur_pos].1;
                let rects: Vec<[f32; 4]> = focusables.iter().map(|f| f.1).collect();
                let dir = if forward {
                    crate::gamepad::Button::DPadRight
                } else {
                    crate::gamepad::Button::DPadLeft
                };
                if let Some(next_idx) = crate::gamepad::focus_step_rect(from, dir, &rects) {
                    let (target, [_, wy, _, wh], is_list) = focusables[next_idx];
                    self.focus = Some(target);
                    self.can_change_value = false;
                    if is_list {
                        if wy < top + 4.0 {
                            self.scroll.amount -= top + 4.0 - wy;
                        } else if wy + wh > bottom - 4.0 {
                            self.scroll.amount += wy + wh - (bottom - 4.0);
                        }
                        self.clamp_scroll(h, content);
                    }
                    return (Vec::new(), true);
                }
            }
            return (Vec::new(), false);
        }

        // 3. Up / Down Arrow keys
        if matches!(key, K::ArrowUp | K::ArrowDown) {
            let down = key == K::ArrowDown;
            if let Some(cur_target) = self.focus
                && let Some(cur_pos) = focusables.iter().position(|&(t, ..)| t == cur_target)
            {
                let from = focusables[cur_pos].1;
                let rects: Vec<[f32; 4]> = focusables.iter().map(|f| f.1).collect();
                let dir = if down {
                    crate::gamepad::Button::DPadDown
                } else {
                    crate::gamepad::Button::DPadUp
                };
                if let Some(next_idx) = crate::gamepad::focus_step_rect(from, dir, &rects) {
                    let (target, [_, wy, _, wh], is_list) = focusables[next_idx];
                    self.focus = Some(target);
                    self.can_change_value = false;
                    if is_list {
                        if wy < top + 4.0 {
                            self.scroll.amount -= top + 4.0 - wy;
                        } else if wy + wh > bottom - 4.0 {
                            self.scroll.amount += wy + wh - (bottom - 4.0);
                        }
                        self.clamp_scroll(h, content);
                    }
                    return (Vec::new(), true);
                }
            } else if !focusables.is_empty() {
                let first = if down { 0 } else { focusables.len() - 1 };
                let (target, [_, wy, _, wh], is_list) = focusables[first];
                self.focus = Some(target);
                self.can_change_value = false;
                if is_list {
                    if wy < top + 4.0 {
                        self.scroll.amount -= top + 4.0 - wy;
                    } else if wy + wh > bottom - 4.0 {
                        self.scroll.amount += wy + wh - (bottom - 4.0);
                    }
                    self.clamp_scroll(h, content);
                }
                return (Vec::new(), true);
            }
            return (Vec::new(), false);
        }

        // 4. Space / Enter / NumpadEnter
        if matches!(key, K::Space | K::Enter | K::NumpadEnter)
            && let Some(target) = self.focus
        {
            match target {
                Target::Item(item) => {
                    if item.slider(s).is_some() {
                        self.can_change_value = !self.can_change_value;
                        return (Vec::new(), true);
                    } else if item == Item::Horizon(H::HdrSettings) {
                        return (vec![Action::ButtonClick, Action::OpenHdr], true);
                    } else {
                        item.cycle(s, shift, framebuffer);
                        return (vec![Action::ButtonClick, Action::Changed], true);
                    }
                }
                Target::Button(action) => {
                    return (vec![Action::ButtonClick, action], true);
                }
            }
        }

        // 5. Escape
        if key == K::Escape {
            if self.can_change_value {
                self.can_change_value = false;
                return (Vec::new(), true);
            }
            return (Vec::new(), false);
        }

        (Vec::new(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdr_settings_open_where_hdr_is_offered() {
        let mut s = Settings::default();
        let mut ui = GraphicsUi::default();
        let size = (480.0, 2000.0);
        let at = |ui: &mut GraphicsUi, s: &Settings, level, item| {
            let (widgets, _, _) = ui.layout(level, s, size.0, size.1);
            let w = widgets
                .iter()
                .find(|w| matches!(w.target, Target::Item(i) if i == item))
                .unwrap();
            (w.x + 1.0, w.y + 1.0)
        };
        let button = at(&mut ui, &s, Level::Advanced, Item::Horizon(H::HdrSettings));
        let click = |ui: &mut GraphicsUi, s: &mut Settings, level, point| {
            ui.click(level, s, point, size, [480, 2000], false, false)
        };
        crate::graphics::offer_hdr(false);
        assert!(click(&mut ui, &mut s, Level::Advanced, button).is_empty());
        // Not even with HDR saved on, from another computer.
        s.horizon.hdr = true;
        assert!(click(&mut ui, &mut s, Level::Advanced, button).is_empty());
        s.horizon.hdr = false;
        crate::graphics::offer_hdr(true);
        assert_eq!(
            click(&mut ui, &mut s, Level::Advanced, button),
            [Action::ButtonClick, Action::OpenHdr]
        );
        // On HDR's screen, the sliders wait for the switch above them.
        let peak = at(&mut ui, &s, Level::Hdr, Item::Horizon(H::HdrPeak));
        assert!(click(&mut ui, &mut s, Level::Hdr, peak).is_empty());
        s.horizon.hdr = true;
        assert_eq!(
            click(&mut ui, &mut s, Level::Hdr, peak),
            [Action::SliderDrag]
        );
        let on = |level, item| {
            extra_rows(level, &s)
                .iter()
                .any(|r| matches!(r, Row::Setting(x) if x.item == Item::Horizon(item)))
        };
        assert!(on(Level::Hdr, H::Hdr) && !on(Level::Advanced, H::Hdr));
        assert!(on(Level::Hdr, H::HdrPeak) && !on(Level::Advanced, H::HdrPeak));
        for item in [H::HdrBlackLevel, H::HdrPaperWhite, H::HdrUiBrightness] {
            assert!(on(Level::Hdr, item) && !on(Level::Advanced, item));
        }
    }
    #[test]
    fn graphics_buttons_sound_on_press_and_sliders_on_release() {
        let mut s = Settings::default();
        let mut ui = GraphicsUi::default();
        let size = (480.0, 320.0);
        let (widgets, _, _) = ui.layout(Level::Advanced, &s, size.0, size.1);
        let button = widgets
            .iter()
            .find(|w| matches!(w.target, Target::Button(_)))
            .unwrap();
        let point = (button.x + 1.0, button.y + 1.0);
        assert!(
            ui.click(
                Level::Advanced,
                &mut s,
                point,
                size,
                [854, 480],
                false,
                true
            )
            .is_empty()
        );
        assert_eq!(
            ui.click(
                Level::Advanced,
                &mut s,
                point,
                size,
                [854, 480],
                false,
                false
            )[0],
            Action::ButtonClick
        );
        assert!(!ui.release());
        let slider = widgets
            .iter()
            .find(|w| matches!(w.target, Target::Item(i) if i.slider(&s).is_some()))
            .unwrap();
        assert_eq!(
            ui.click(
                Level::Advanced,
                &mut s,
                (slider.x + 1.0, slider.y + 1.0),
                size,
                [854, 480],
                false,
                false
            ),
            vec![Action::SliderDrag]
        );
        assert_eq!(
            ui.click(
                Level::Advanced,
                &mut s,
                (-100.0, -100.0),
                size,
                [854, 480],
                true,
                false
            ),
            vec![Action::SliderDrag]
        );
        assert!(ui.release());
        assert!(!ui.release());
    }

    #[test]
    fn scrolling_stops_at_both_limits_before_layout() {
        let s = Settings::default();
        for level in [Level::Simple, Level::Advanced] {
            let mut ui = GraphicsUi::default();
            let h = 2.0 * STYLE.band + 10.0;
            let (top, bottom) = GraphicsUi::bounds(h);
            let max = GraphicsUi::content_height(level, &s) - (bottom - top);
            assert!(max > 4.5);
            for _ in 0..3 {
                ui.scroll(level, &s, h, 1000.0);
                assert_eq!(ui.scroll.amount, 0.0);
            }
            for _ in 0..3 {
                ui.scroll(level, &s, h, -1000.0);
                assert_eq!(ui.scroll.amount, max);
            }
            ui.scroll(level, &s, h, 0.25);
            assert_eq!(ui.scroll.amount, max - 4.5);
            ui.scroll(level, &s, h, 1000.0);
            ui.scroll(level, &s, h, -0.25);
            assert_eq!(ui.scroll.amount, 4.5);
        }
    }

    #[test]
    fn the_scrollbar_can_be_held_and_dragged() {
        // #3552: AbstractScrollArea.updateScrolling holds the scrollbar on a press over its
        // column, mouseDragged moves the list with the pointer, onRelease lets go.
        let mut s = Settings::default();
        let size = (480.0, 2.0 * STYLE.band + 60.0);
        let (top, bottom) = GraphicsUi::bounds(size.1);
        // On the scrollbar column, as `draw` paints it.
        let x =
            STYLE.row_x(size.0) + STYLE.control_width + STYLE.gap + STYLE.text_width(size.0) + 8.0;
        let first_row = |ui: &mut GraphicsUi| {
            ui.layout(Level::Advanced, &Settings::default(), size.0, size.1)
                .0[0]
                .y
        };
        let mut ui = GraphicsUi::default();
        let start = first_row(&mut ui);
        let at = |y| (x, y);
        let press = |ui: &mut GraphicsUi, s: &mut Settings, y, drag| {
            ui.click(Level::Advanced, s, at(y), size, [854, 480], drag, false)
        };
        // The press itself makes no sound and changes nothing.
        assert!(press(&mut ui, &mut s, top + 5.0, false).is_empty());
        assert!(press(&mut ui, &mut s, top + 15.0, true).is_empty());
        let moved = first_row(&mut ui);
        assert!(moved < start, "dragging down scrolls the list down");
        // Past the bottom of the list: the last row.
        press(&mut ui, &mut s, bottom + 50.0, true);
        let content = GraphicsUi::content_height(Level::Advanced, &s);
        assert_eq!(first_row(&mut ui), start - (content - (bottom - top)));
        // Above the list: the first row again.
        press(&mut ui, &mut s, top - 50.0, true);
        assert_eq!(first_row(&mut ui), start);
        // Released: drags no longer scroll, and no slider click plays.
        assert!(!ui.release());
        press(&mut ui, &mut s, bottom + 50.0, true);
        assert_eq!(first_row(&mut ui), start);
    }

    #[test]
    fn resizing_clamps_before_positioning_graphics_rows() {
        let s = Settings::default();
        for level in [Level::Simple, Level::Advanced] {
            let mut ui = GraphicsUi::default();
            ui.scroll(level, &s, 2.0 * STYLE.band + 10.0, -1000.0);
            assert!(ui.scroll.amount > 0.0);
            let (widgets, headers, _) = ui.layout(level, &s, 480.0, 4000.0);
            assert_eq!(ui.scroll.amount, 0.0);
            if let Some((y, _)) = headers.first() {
                assert_eq!(*y, STYLE.band + 4.0);
            }
            let (expected, _, _) = GraphicsUi::default().layout(level, &s, 480.0, 4000.0);
            assert_eq!(
                widgets.iter().map(|w| w.y).collect::<Vec<_>>(),
                expected.iter().map(|w| w.y).collect::<Vec<_>>()
            );
            ui.scroll(level, &s, 4000.0, -1000.0);
            assert_eq!(ui.scroll.amount, 0.0);
        }
    }

    #[test]
    fn classic_submenus_only_add_horizon_settings_without_empty_groups() {
        for level in [Level::Simple, Level::Advanced] {
            let rows = extra_rows(level, &Settings::default());
            assert!(rows.iter().any(|row| matches!(row, Row::Setting(_))));
            for (i, row) in rows.iter().enumerate() {
                match row {
                    Row::Setting(s) => assert!(matches!(s.item, Item::Horizon(_))),
                    Row::Header(_) => assert!(matches!(rows.get(i + 1), Some(Row::Setting(_)))),
                }
            }
            assert!(
                super::rows(level).iter().any(
                    |row| matches!(row, Row::Setting(s) if matches!(s.item, Item::Vanilla(_)))
                )
            );
        }
    }

    #[test]
    fn every_advanced_setting_has_a_description() {
        for level in [Level::Simple, Level::Advanced] {
            for row in rows(level) {
                if let Row::Setting(s) = row {
                    assert!(!s.text.is_empty(), "{:?}", s.item);
                    assert!(s.text.len() < 80, "one line: {:?}", s.item);
                }
            }
        }
    }

    #[test]
    fn lod_distance_cycles_in_both_levels_without_changing_near_distance() {
        let mut s = Settings::default();
        let item = Item::Horizon(H::LodDistance);
        let near = s.render_distance;
        assert_eq!(item.label(&s).plain(), "LOD Distance: Off");
        for level in [Level::Simple, Level::Advanced] {
            let items: Vec<_> = rows(level)
                .into_iter()
                .filter_map(|row| match row {
                    Row::Setting(setting) => Some(setting.item),
                    Row::Header(_) => None,
                })
                .collect();
            let render = items
                .iter()
                .position(|i| *i == Item::Vanilla(Opt::RenderDistance))
                .unwrap();
            assert_eq!(items[render + 1], item);
        }
        for distance in [64, 128, 256, 512, 1024, 0] {
            item.cycle(&mut s, false, [854, 480]);
            assert_eq!(s.horizon.lod_distance, distance);
            assert_eq!(s.render_distance, near);
        }
        item.cycle(&mut s, true, [854, 480]);
        assert_eq!(s.horizon.lod_distance, 1024);
        assert_eq!(item.label(&s).plain(), "LOD Distance: 1024 chunks");
        assert_eq!(
            Item::Vanilla(Opt::RenderDistance).label(&s).plain(),
            format!("Full Detail Distance: {near} chunks")
        );
    }

    #[test]
    fn presets_set_vanilla_and_horizon_options() {
        let mut s = Settings::default();
        Item::Horizon(H::Preset).cycle(&mut s, false, [854, 480]);
        assert_eq!(s.horizon.preset, Preset::Performance);
        assert_eq!(s.render_distance, 8);
        assert_eq!(s.clouds, "false");
        // Any quality change leaves the preset.
        Item::Vanilla(Opt::Clouds).cycle(&mut s, false, [854, 480]);
        assert_eq!(s.horizon.preset, Preset::Custom);
        // Display preferences keep it.
        Item::Horizon(H::Preset).cycle(&mut s, true, [854, 480]);
        Item::Vanilla(Opt::Vsync).cycle(&mut s, false, [854, 480]);
        assert_eq!(s.horizon.preset, Preset::Ultra);
    }

    #[test]
    fn upload_budget_slider_steps() {
        let mut s = Settings::default();
        H::UploadBudget.set_slider(&mut s, 1.0);
        assert_eq!(s.horizon.upload_budget_kib, 16384);
        assert_eq!(H::UploadBudget.slider(&s), Some(1.0));
        H::UploadBudget.set_slider(&mut s, 0.0);
        assert_eq!(s.horizon.upload_budget_kib, 0);
    }

    #[test]
    fn antialiasing_cycles_off_fxaa_taa() {
        let mut s = Settings::default();
        let item = Item::Horizon(H::Antialiasing);
        for label in ["FXAA", "TAA", "Off"] {
            item.cycle(&mut s, false, [854, 480]);
            assert_eq!(item.label(&s).plain(), format!("Anti-aliasing: {label}"));
        }
        item.cycle(&mut s, true, [854, 480]);
        assert_eq!((s.horizon.fxaa, s.horizon.taa), (false, true));

        // Upscaling needs TAA: a ratio turns it on, leaving TAA goes back to Native.
        s.horizon.taa = false;
        let upscaling = Item::Horizon(H::Upscaling);
        upscaling.cycle(&mut s, false, [854, 480]);
        assert_eq!(upscaling.label(&s).plain(), "Upscaling: Quality");
        assert!(s.horizon.taa);
        item.cycle(&mut s, false, [854, 480]);
        assert_eq!(s.horizon.upscaling, Upscaling::Native);

        // The Upscaling row is only on the screen while TAA is on.
        let shown = |s: &Settings| {
            extra_rows(Level::Advanced, s)
                .iter()
                .any(|row| matches!(row, Row::Setting(r) if r.item == upscaling))
        };
        assert!(!shown(&s));
        s.horizon.taa = true;
        assert!(shown(&s));
    }

    #[test]
    fn entity_culling_is_opt_in_and_toggles_from_advanced_graphics() {
        let mut s = Settings::default();
        let item = Item::Horizon(H::EntityCulling);
        assert_eq!(item.switch(&s), Some(false));
        assert_eq!(item.label(&s).plain(), "Entity Culling: Off");
        for enabled in [false, true] {
            s.horizon.entity_culling = enabled;
            for preset in Preset::ALL {
                crate::graphics::apply_preset(preset, &mut s);
                assert_eq!(s.horizon.entity_culling, enabled);
            }
        }

        let mut ui = GraphicsUi::default();
        let size = (480.0, 4000.0);
        let (widgets, _, _) = ui.layout(Level::Advanced, &s, size.0, size.1);
        let switch = widgets
            .iter()
            .find(|w| matches!(w.target, Target::Item(i) if i == item))
            .unwrap();
        let point = (switch.x + 1.0, switch.y + 1.0);
        for enabled in [false, true] {
            s.horizon.preset = Preset::Balanced;
            assert_eq!(
                ui.click(
                    Level::Advanced,
                    &mut s,
                    point,
                    size,
                    [854, 480],
                    false,
                    false
                ),
                vec![Action::ButtonClick, Action::Changed]
            );
            assert_eq!(item.switch(&s), Some(enabled));
            assert_eq!(s.horizon.preset, Preset::Custom);
        }
        assert_eq!(item.label(&s).plain(), "Entity Culling: On");
    }

    #[test]
    fn graphics_slider_keyboard_navigation_and_selection_toggle_parity() {
        use winit::keyboard::KeyCode as K;
        let mut s = Settings::default();
        let mut ui = GraphicsUi::default();
        let size = (480.0, 600.0);
        let fb = [854, 480];

        s.horizon.chunk_updates = 10;
        s.horizon.resolution_scale = 100;

        // 1. Tab navigates down to Chunk Updates slider (in Level::Advanced)
        let mut found_chunk_updates = false;
        for _ in 0..15 {
            ui.key_modifiers(Level::Advanced, &mut s, K::Tab, false, size, fb);
            if ui.focus == Some(Target::Item(Item::Horizon(H::ChunkUpdates))) {
                found_chunk_updates = true;
                break;
            }
        }
        assert!(found_chunk_updates);
        assert!(
            ui.can_change_value,
            "Tab onto slider activates can_change_value"
        );

        // 2. Right Arrow steps chunk updates from 10 to 11
        let (actions, handled) =
            ui.key_modifiers(Level::Advanced, &mut s, K::ArrowRight, false, size, fb);
        assert!(handled);
        assert_eq!(actions, vec![Action::SliderDrag]);
        assert_eq!(s.horizon.chunk_updates, 11);

        // 3. Left Arrow steps back to 10
        ui.key_modifiers(Level::Advanced, &mut s, K::ArrowLeft, false, size, fb);
        assert_eq!(s.horizon.chunk_updates, 10);

        // 4. Space toggles can_change_value to false
        ui.key_modifiers(Level::Advanced, &mut s, K::Space, false, size, fb);
        assert!(!ui.can_change_value);

        // 5. Space toggles can_change_value back to true
        ui.key_modifiers(Level::Advanced, &mut s, K::Enter, false, size, fb);
        assert!(ui.can_change_value);

        // 6. Escape while can_change_value is true cancels it without closing
        let (actions, handled) =
            ui.key_modifiers(Level::Advanced, &mut s, K::Escape, false, size, fb);
        assert!(handled);
        assert!(actions.is_empty());
        assert!(!ui.can_change_value);

        // 8. Tab to ResolutionScale (steps by 5%)
        let mut found_res = false;
        for _ in 0..20 {
            ui.key_modifiers(Level::Advanced, &mut s, K::Tab, false, size, fb);
            if ui.focus == Some(Target::Item(Item::Horizon(H::ResolutionScale))) {
                found_res = true;
                break;
            }
        }
        assert!(found_res);
        assert!(ui.can_change_value);
        ui.key_modifiers(Level::Advanced, &mut s, K::ArrowRight, false, size, fb);
        assert_eq!(s.horizon.resolution_scale, 105);
        ui.key_modifiers(Level::Advanced, &mut s, K::ArrowLeft, false, size, fb);
        assert_eq!(s.horizon.resolution_scale, 100);
    }

    #[test]
    fn graphics_slider_and_cycle_mouse_wheel() {
        let mut s = Settings::default();
        let mut ui = GraphicsUi::default();
        let size = (480.0, 600.0);
        let fb = [854, 480];

        // 1. Wheel on a Horizon slider: Chunk Updates in Level::Advanced
        s.horizon.chunk_updates = 16;
        let (widgets, _, _) = ui.layout(Level::Advanced, &s, size.0, size.1);
        let cu = widgets
            .iter()
            .find(|w| matches!(w.target, Target::Item(Item::Horizon(H::ChunkUpdates))))
            .unwrap();
        let cursor = (cu.x + 5.0, cu.y + 5.0);

        // Scroll up (+1.0) increases Chunk Updates
        let actions = ui.wheel(Level::Advanced, &mut s, cursor, size, fb, 1.0);
        assert_eq!(actions, vec![Action::SliderDrag]);
        assert_eq!(s.horizon.chunk_updates, 17);

        // Scroll down (-1.0) decreases Chunk Updates
        let actions = ui.wheel(Level::Advanced, &mut s, cursor, size, fb, -1.0);
        assert_eq!(actions, vec![Action::SliderDrag]);
        assert_eq!(s.horizon.chunk_updates, 16);

        // 2. Wheel on a Cycle button: Far Detail
        let (widgets, _, _) = ui.layout(Level::Advanced, &s, size.0, size.1);
        let far = widgets
            .iter()
            .find(|w| matches!(w.target, Target::Item(Item::Horizon(H::FarDetail))))
            .unwrap();
        let cursor = (far.x + 5.0, far.y + 5.0);
        s.horizon.far_detail = FarDetail::Full;
        let actions = ui.wheel(Level::Advanced, &mut s, cursor, size, fb, 1.0);
        assert_eq!(actions, vec![Action::ButtonClick, Action::Changed]);
        assert_eq!(s.horizon.far_detail, FarDetail::Reduced);
    }
}
