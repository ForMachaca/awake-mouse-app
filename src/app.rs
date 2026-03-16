use std::collections::VecDeque;
use std::fs;
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;

use eframe::egui::{
    self, pos2, vec2, Align, Button, Color32, Context, CornerRadius, DragValue, FontData,
    FontDefinitions, FontFamily, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Ui,
};

use crate::engine::MotionConfig;
use crate::worker::{spawn_worker, WorkerEvent, WorkerHandle};

const LOG_LIMIT: usize = 12;
const CJK_FONT_NAME: &str = "system-cjk";

const BG: Color32 = Color32::from_rgb(6, 12, 18);
const SURFACE: Color32 = Color32::from_rgb(10, 20, 30);
const SURFACE_RAISED: Color32 = Color32::from_rgb(14, 27, 40);
const STROKE_SUBTLE: Color32 = Color32::from_rgb(40, 68, 92);
const STROKE_BRIGHT: Color32 = Color32::from_rgb(83, 191, 206);
const ACCENT: Color32 = Color32::from_rgb(93, 222, 239);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(229, 242, 247);
const TEXT_SECONDARY: Color32 = Color32::from_rgb(151, 177, 193);
const TEXT_MUTED: Color32 = Color32::from_rgb(108, 131, 149);
const DANGER: Color32 = Color32::from_rgb(166, 84, 92);

pub fn configure_theme(ctx: &Context) {
    install_cjk_font(ctx);

    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.faint_bg_color = SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(4, 10, 15);
    visuals.code_bg_color = SURFACE;
    visuals.selection.bg_fill = accent_soft();
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.window_corner_radius = CornerRadius::same(24);
    visuals.menu_corner_radius = CornerRadius::same(20);
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STROKE_SUBTLE);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(18);
    visuals.widgets.inactive.bg_fill = SURFACE_RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, STROKE_SUBTLE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(16);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(18, 38, 54);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, STROKE_BRIGHT);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(16);
    visuals.widgets.active.bg_fill = Color32::from_rgb(28, 56, 75);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2, ACCENT);
    visuals.widgets.active.corner_radius = CornerRadius::same(16);
    visuals.widgets.open.bg_fill = SURFACE_RAISED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, STROKE_BRIGHT);
    visuals.widgets.open.corner_radius = CornerRadius::same(16);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
    ctx.set_visuals(visuals);
}

fn install_cjk_font(ctx: &Context) {
    let Some(font_bytes) = load_system_cjk_font() else {
        return;
    };

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        CJK_FONT_NAME.to_owned(),
        Arc::new(FontData::from_owned(font_bytes)),
    );

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, CJK_FONT_NAME.to_owned());
    }

    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push(CJK_FONT_NAME.to_owned());
    }

    ctx.set_fonts(fonts);
}

fn load_system_cjk_font() -> Option<Vec<u8>> {
    preferred_cjk_font_paths()
        .iter()
        .find_map(|path| fs::read(path).ok())
}

#[cfg(target_os = "macos")]
fn preferred_cjk_font_paths() -> &'static [&'static str] {
    &[
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
    ]
}

#[cfg(target_os = "windows")]
fn preferred_cjk_font_paths() -> &'static [&'static str] {
    &[
        "C:/Windows/Fonts/arialuni.ttf",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/msyh.ttf",
    ]
}

#[cfg(target_os = "linux")]
fn preferred_cjk_font_paths() -> &'static [&'static str] {
    &[
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansSC-Regular.otf",
        "/usr/share/fonts/opentype/source-han-sans/SourceHanSansSC-Regular.otf",
    ]
}

pub struct AwakeMouseApp {
    worker: WorkerHandle,
    config: FormState,
    backend_label: String,
    status_line: String,
    running: bool,
    logs: VecDeque<String>,
}

impl Default for AwakeMouseApp {
    fn default() -> Self {
        let worker = spawn_worker();
        let mut logs = VecDeque::new();
        logs.push_back("应用已启动，等待原生后端就绪。".to_owned());

        Self {
            worker,
            config: FormState::default(),
            backend_label: "初始化中".to_owned(),
            status_line: "尚未开始".to_owned(),
            running: false,
            logs,
        }
    }
}

impl eframe::App for AwakeMouseApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_events();
        ctx.request_repaint_after(std::time::Duration::from_millis(200));

        egui::CentralPanel::default()
            .frame(Frame::new().fill(BG).inner_margin(Margin::same(20)))
            .show(ctx, |ui| {
                paint_background(ui);
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        self.header(ui);
                        ui.add_space(12.0);
                        self.control_grid(ui);
                        ui.add_space(12.0);
                        self.log_panel(ui);
                        ui.add_space(8.0);
                    });
            });
    }
}

impl AwakeMouseApp {
    fn poll_events(&mut self) {
        loop {
            match self.worker.try_recv() {
                Ok(event) => self.handle_event(event),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.running = false;
                    self.status_line = "后台线程已退出".to_owned();
                    self.push_log("后台线程已断开。");
                    break;
                }
            }
        }
    }

    fn handle_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::BackendReady(label) => {
                self.backend_label = label;
                self.push_log("原生后端已就绪。");
            }
            WorkerEvent::RunningChanged(running) => {
                self.running = running;
                self.status_line = if running {
                    "运行中：已启用防休眠与鼠标轨迹".to_owned()
                } else {
                    "已停止：防休眠已释放".to_owned()
                };
            }
            WorkerEvent::Info(message) => {
                self.status_line = message.clone();
                self.push_log(message);
            }
            WorkerEvent::Error(message) => {
                self.running = false;
                self.status_line = format!("错误：{message}");
                self.push_log(format!("错误：{message}"));
            }
        }
    }

    fn header(&self, ui: &mut Ui) {
        panel_frame(surface_soft(), Stroke::new(1.0, STROKE_SUBTLE))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        overline(ui, "科技极简控制台");
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("屏幕常醒")
                                .size(24.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        ui.add_space(4.0);
                        ui.label(
                    RichText::new(
                                "使用系统原生 API 保持屏幕处于唤醒状态，并通过带随机性的平滑轨迹发送真实鼠标移动事件。",
                            )
                            .size(13.0)
                            .color(TEXT_SECONDARY),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        status_pill(ui, self.running);
                    });
                });

                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    summary_tile(ui, "原生后端", &self.backend_label);
                    summary_tile(
                        ui,
                        "当前状态",
                        if self.running { "防休眠运行中" } else { "等待启动" },
                    );
                    summary_tile(ui, "默认间隔", "20 秒");
                });
            });
    }

    fn control_grid(&mut self, ui: &mut Ui) {
        ui.columns(2, |columns| {
            let (left_columns, right_columns) = columns.split_at_mut(1);
            let left = &mut left_columns[0];
            let right = &mut right_columns[0];

            panel_frame(SURFACE, Stroke::new(1.0, STROKE_SUBTLE)).show(left, |ui| {
                overline(ui, "参数");
                ui.add_space(4.0);
                ui.label(section_title("运行配置"));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("建议将间隔保持在 20 至 90 秒之间，既能防休眠，也不会让指针过于频繁地扰动桌面。")
                        .size(13.0)
                        .color(TEXT_SECONDARY),
                );

                ui.add_space(18.0);
                parameter_card(
                    ui,
                    "执行间隔",
                    "单位：秒",
                    DragValue::new(&mut self.config.interval_secs)
                        .range(5.0..=3600.0)
                        .speed(1.0),
                );
                ui.add_space(10.0);
                parameter_card(
                    ui,
                    "移动幅度",
                    "单位：像素",
                    DragValue::new(&mut self.config.travel_px)
                        .range(2.0..=120.0)
                        .speed(1.0),
                );
                ui.add_space(10.0);
                parameter_card(
                    ui,
                    "轨迹时长",
                    "单位：毫秒",
                    DragValue::new(&mut self.config.duration_ms)
                        .range(120_u64..=5000_u64)
                        .speed(10.0),
                );

                ui.add_space(14.0);
                subtle_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.config.return_to_origin, "轨迹结束后回到起点");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new("默认关闭")
                                    .size(12.0)
                                    .color(TEXT_MUTED),
                            );
                        });
                    });
                });
            });

            panel_frame(SURFACE, Stroke::new(1.0, STROKE_SUBTLE)).show(right, |ui| {
                overline(ui, "控制");
                ui.add_space(4.0);
                ui.label(section_title("运行面板"));
                ui.add_space(6.0);

                subtle_frame().show(ui, |ui| {
                    ui.label(
                        RichText::new("首次运行前，请确认已授予系统辅助功能权限，尤其是 macOS。")
                            .size(13.0)
                            .color(TEXT_SECONDARY),
                    );
                });

                ui.add_space(16.0);
                let config = self.config.to_motion_config();

                if !self.running {
                    if primary_button(ui, "启动防休眠").clicked() {
                        if let Err(err) = self.worker.start(config) {
                            self.push_log(format!("发送启动命令失败：{err}"));
                        }
                    }
                } else {
                    if secondary_button(ui, "更新当前参数").clicked() {
                        if let Err(err) = self.worker.update(config) {
                            self.push_log(format!("发送更新命令失败：{err}"));
                        }
                    }

                    ui.add_space(10.0);

                    if danger_button(ui, "停止并释放防休眠").clicked() {
                        if let Err(err) = self.worker.stop() {
                            self.push_log(format!("发送停止命令失败：{err}"));
                        }
                    }
                }

                ui.add_space(18.0);
                subtle_frame().show(ui, |ui| {
                    overline(ui, "执行特征");
                    ui.add_space(8.0);
                    info_row(ui, "轨迹模式", "随机微扰曲线逐帧移动");
                    ui.add_space(6.0);
                    info_row(ui, "系统调用", "原生防休眠 + 原生鼠标事件");
                    ui.add_space(6.0);
                    info_row(
                        ui,
                        "当前策略",
                        if self.config.return_to_origin {
                            "随机移动后回到原点"
                        } else {
                            "随机移动后停留新位置"
                        },
                    );
                });
            });
        });
    }

    fn log_panel(&mut self, ui: &mut Ui) {
        panel_frame(SURFACE, Stroke::new(1.0, STROKE_SUBTLE)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    overline(ui, "日志");
                    ui.add_space(4.0);
                    ui.label(section_title("最近事件"));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("保留 {} 条", LOG_LIMIT))
                            .size(12.0)
                            .color(TEXT_MUTED),
                    );
                });
            });

            ui.add_space(14.0);

            for entry in self.logs.iter().rev() {
                subtle_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(ACCENT, "●");
                        ui.label(RichText::new(entry).size(13.0).color(TEXT_PRIMARY));
                    });
                });
                ui.add_space(8.0);
            }
        });
    }

    fn push_log(&mut self, message: impl Into<String>) {
        self.logs.push_back(message.into());
        while self.logs.len() > LOG_LIMIT {
            self.logs.pop_front();
        }
    }
}

impl Drop for AwakeMouseApp {
    fn drop(&mut self) {
        let _ = self.worker.shutdown();
    }
}

#[derive(Clone, Copy)]
struct FormState {
    interval_secs: f32,
    travel_px: f32,
    duration_ms: u64,
    return_to_origin: bool,
}

impl Default for FormState {
    fn default() -> Self {
        Self {
            interval_secs: 20.0,
            travel_px: 18.0,
            duration_ms: 900,
            return_to_origin: false,
        }
    }
}

impl FormState {
    fn to_motion_config(self) -> MotionConfig {
        MotionConfig {
            interval: std::time::Duration::from_secs_f64(self.interval_secs as f64),
            travel_px: self.travel_px as f64,
            duration: std::time::Duration::from_millis(self.duration_ms),
            return_to_origin: self.return_to_origin,
        }
    }
}

fn paint_background(ui: &Ui) {
    let rect = ui.max_rect();
    let painter = ui.painter();

    painter.rect_filled(rect, CornerRadius::ZERO, BG);

    let glow_rect = egui::Rect::from_min_size(
        rect.left_top() + vec2(24.0, 18.0),
        vec2(rect.width() * 0.52, 180.0),
    );
    painter.rect_filled(glow_rect, CornerRadius::same(28), accent_soft());

    let grid_spacing = 72.0;
    let mut x = rect.left() + 16.0;
    while x < rect.right() {
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(0.6, Color32::from_rgba_unmultiplied(50, 90, 114, 40)),
        );
        x += grid_spacing;
    }

    let mut y = rect.top() + 14.0;
    while y < rect.bottom() {
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(0.6, Color32::from_rgba_unmultiplied(50, 90, 114, 28)),
        );
        y += grid_spacing;
    }

    painter.line_segment(
        [
            pos2(rect.left() + 40.0, rect.top() + 36.0),
            pos2(rect.left() + 240.0, rect.top() + 36.0),
        ],
        Stroke::new(1.2, Color32::from_rgba_unmultiplied(93, 222, 239, 120)),
    );
}

fn panel_frame(fill: Color32, stroke: Stroke) -> Frame {
    Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(24))
        .inner_margin(Margin::same(14))
}

fn surface_soft() -> Color32 {
    Color32::from_rgba_unmultiplied(19, 40, 58, 160)
}

fn accent_soft() -> Color32 {
    Color32::from_rgba_unmultiplied(93, 222, 239, 28)
}

fn subtle_frame() -> Frame {
    Frame::new()
        .fill(SURFACE_RAISED)
        .stroke(Stroke::new(1.0, STROKE_SUBTLE))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::same(12))
}

fn summary_tile(ui: &mut Ui, title: &str, value: &str) {
    subtle_frame().show(ui, |ui| {
        ui.set_min_width(152.0);
        ui.label(RichText::new(title).size(11.0).color(TEXT_MUTED));
        ui.add_space(3.0);
        ui.label(RichText::new(value).size(13.0).strong().color(TEXT_PRIMARY));
    });
}

fn parameter_card<'a>(ui: &mut Ui, title: &str, unit: &str, widget: impl egui::Widget + 'a) {
    subtle_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(14.0).strong().color(TEXT_PRIMARY));
                ui.label(RichText::new(unit).size(12.0).color(TEXT_MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_sized([88.0, 34.0], widget);
            });
        });
    });
}

fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 46.0],
        Button::new(
            RichText::new(text)
                .size(15.0)
                .strong()
                .color(Color32::from_rgb(5, 16, 21)),
        )
        .fill(ACCENT),
    )
}

fn secondary_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 44.0],
        Button::new(RichText::new(text).size(14.0).strong().color(TEXT_PRIMARY))
            .fill(Color32::from_rgb(16, 36, 50)),
    )
}

fn danger_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 44.0],
        Button::new(RichText::new(text).size(14.0).strong().color(TEXT_PRIMARY)).fill(DANGER),
    )
}

fn status_pill(ui: &mut Ui, running: bool) {
    let fill = if running {
        Color32::from_rgba_unmultiplied(93, 222, 239, 26)
    } else {
        Color32::from_rgba_unmultiplied(120, 141, 159, 24)
    };
    let stroke = if running {
        Stroke::new(1.0, STROKE_BRIGHT)
    } else {
        Stroke::new(1.0, STROKE_SUBTLE)
    };

    Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(249))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(if running { ACCENT } else { TEXT_MUTED }, "●");
                ui.label(
                    RichText::new(if running { "运行中" } else { "空闲中" })
                        .size(12.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
            });
        });
}

fn info_row(ui: &mut Ui, title: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).size(12.5).color(TEXT_MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.5).color(TEXT_PRIMARY));
        });
    });
}

fn overline(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(ACCENT).strong());
}

fn section_title(text: &str) -> RichText {
    RichText::new(text).size(18.0).color(TEXT_PRIMARY).strong()
}
