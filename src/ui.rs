use eframe::egui;

use crate::app::{self, AppState};
use crate::db;
use crate::export;
use crate::idle::IdleEvent;
use crate::stats;
use crate::tracker::TrackerState;
use crate::tray;
use crate::util;

#[derive(PartialEq)]
pub(crate) enum Tab {
    Tasks,
    Journal,
    Report,
}

pub struct ChronosApp {
    pub state: AppState,
    pub tray_ctx: tray::TrayContext,
    active_tab: Tab,
    pub new_task_name: String,
    pub show_add_dialog: bool,
    pub show_archived: bool,
    pub selected_task_id: Option<i64>,
    pub rename_buf: String,
    pub renaming_task_id: Option<i64>,
    pub filter_date_from: String,
    pub filter_date_to: String,
    pub filter_active: bool,
    pub search_query: String,
    pub report_from: String,
    pub report_to: String,
    pub show_help: bool,
    pub show_logs: bool,
    needs_visibility_update: bool,
    confirm_delete_task_id: Option<i64>,
    tray_event_rx: Option<std::sync::mpsc::Receiver<tray_icon::menu::MenuEvent>>,
}

impl ChronosApp {
    pub fn new(state: AppState, tray_ctx: tray::TrayContext) -> Self {
        let today = util::ts_to_date(stats::now_ts());
        Self {
            state,
            tray_ctx,
            active_tab: Tab::Tasks,
            new_task_name: String::new(),
            show_add_dialog: false,
            show_archived: false,
            selected_task_id: None,
            rename_buf: String::new(),
            renaming_task_id: None,
            filter_date_from: today.clone(),
            filter_date_to: today.clone(),
            filter_active: false,
            search_query: String::new(),
            report_from: today.clone(),
            report_to: today.clone(),
            show_help: false,
            show_logs: false,
            needs_visibility_update: false,
            confirm_delete_task_id: None,
            tray_event_rx: None,
        }
    }

    fn process_tray_events(&mut self) {
        if let Some(rx) = &self.tray_event_rx {
            while let Ok(event) = rx.try_recv() {
                let id = event.id.0.as_str();
                match id {
                    tray::MENU_ID_START => {
                        if let Some(tid) = self.selected_task_id.or(self.state.current_task_id) {
                            let name = self
                                .state
                                .db
                                .lock()
                                .ok()
                                .and_then(|db| db::get_task(&db, tid).ok().flatten())
                                .map(|t| t.name)
                                .unwrap_or_default();
                            let _ = self.state.start_tracking(tid, &name);
                        }
                    }
                    tray::MENU_ID_STOP => {
                        let _ = self.state.stop_tracking();
                    }
                    tray::MENU_ID_PAUSE => {
                        let _ = self.state.pause_tracking();
                    }
                    tray::MENU_ID_RESUME => {
                        let _ = self.state.resume_tracking();
                    }
                    tray::MENU_ID_TOGGLE => {
                        self.state.toggle_window();
                        self.needs_visibility_update = true;
                    }
                    tray::MENU_ID_LOGS => {
                        self.show_logs = true;
                        self.state.window_visible = true;
                        self.needs_visibility_update = true;
                    }
                    tray::MENU_ID_QUIT => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
                self.state.update_status();
                self.update_tray_tooltip();
            }
        }
    }

    fn update_tray_tooltip(&self) {
        let tip = match self.state.tracker_state() {
            TrackerState::Idle => "Chronos \u{2014} Idle".to_string(),
            TrackerState::Running => {
                let secs = self.state.elapsed_seconds();
                format!("Chronos \u{2014} Tracking ({})", app::format_duration(secs))
            }
            TrackerState::Paused => {
                let secs = self.state.elapsed_seconds();
                format!("Chronos \u{2014} Paused ({})", app::format_duration(secs))
            }
        };
        self.tray_ctx.tray.set_tooltip(Some(tip)).ok();
    }

    fn update_menu_states(&self) {
        let state = self.state.tracker_state();

        let status_text = match state {
            TrackerState::Idle => "Status: Idle".to_string(),
            TrackerState::Running => {
                let secs = self.state.elapsed_seconds();
                let name = self
                    .state
                    .current_task_name
                    .as_deref()
                    .unwrap_or("Unknown Task");
                format!("Tracking: {} ({})", name, app::format_duration(secs))
            }
            TrackerState::Paused => {
                let secs = self.state.elapsed_seconds();
                let name = self
                    .state
                    .current_task_name
                    .as_deref()
                    .unwrap_or("Unknown Task");
                format!("Paused: {} ({})", name, app::format_duration(secs))
            }
        };
        self.tray_ctx.items.status.set_text(status_text);

        self.tray_ctx
            .items
            .start
            .set_enabled(state == TrackerState::Idle || state == TrackerState::Paused);
        self.tray_ctx
            .items
            .stop
            .set_enabled(state != TrackerState::Idle);
        self.tray_ctx
            .items
            .pause
            .set_enabled(state == TrackerState::Running);
        self.tray_ctx
            .items
            .resume
            .set_enabled(state == TrackerState::Paused);
    }

    fn render_task_tree(&mut self, ui: &mut egui::Ui) {
        let tasks = self
            .state
            .db
            .lock()
            .ok()
            .and_then(|db| db::get_all_tasks(&db).ok())
            .unwrap_or_default();

        let q = self.search_query.trim().to_lowercase();
        let matches = |t: &db::TaskRow| -> bool {
            if q.is_empty() {
                true
            } else {
                t.name.to_lowercase().contains(&q)
            }
        };

        let roots: Vec<db::TaskRow> = tasks
            .iter()
            .filter(|t| {
                t.parent_id.is_none()
                    && (self.show_archived || !t.is_archived)
                    && (q.is_empty()
                        || matches(t)
                        || tasks.iter().any(|c| {
                            c.parent_id == Some(t.id)
                                && (matches(c)
                                    || tasks
                                        .iter()
                                        .any(|gc| gc.parent_id == Some(c.id) && matches(gc)))
                        }))
            })
            .cloned()
            .collect();

        for root in &roots {
            self.render_task_node(ui, root, &tasks, 0);
        }
    }

    fn render_task_node(
        &mut self,
        ui: &mut egui::Ui,
        task: &db::TaskRow,
        all_tasks: &[db::TaskRow],
        depth: usize,
    ) {
        let indent = 16.0 * depth as f32;
        ui.horizontal(|ui| {
            ui.allocate_space(egui::vec2(indent, 0.0));

            let icon = if task.is_project {
                "\u{1F4C1}"
            } else {
                "\u{1F4CB}"
            };
            let is_active = self.state.current_task_id == Some(task.id);
            let is_selected = self.selected_task_id == Some(task.id);

            let prefix = if is_active { "\u{25B6} " } else { "" };
            let paid_marker = if !task.is_payable { " \u{2697}" } else { "" };
            let archived_s = if task.is_archived { " \u{1F4E6}" } else { "" };

            let total_secs = self
                .state
                .db
                .lock()
                .ok()
                .and_then(|db| db::get_total_duration_for_task(&db, task.id).ok())
                .unwrap_or(0);
            let time_str = app::format_duration(total_secs as u64);

            let label = format!(
                "{prefix}{icon} {}{paid_marker}{archived_s}  [{}]",
                task.name, time_str
            );

            let response = if is_selected {
                ui.selectable_label(true, label)
            } else {
                ui.selectable_label(false, label)
            };

            if response.clicked() {
                self.selected_task_id = Some(task.id);
            }
            if response.double_clicked() {
                self.selected_task_id = Some(task.id);
                let name = task.name.clone();
                let _ = self.state.start_tracking(task.id, &name);
            }

            if is_active {
                response.highlight();
            }

            if !is_active
                && ui
                    .small_button("\u{25B6}")
                    .on_hover_text("Start tracking")
                    .clicked()
            {
                let name = task.name.clone();
                let _ = self.state.start_tracking(task.id, &name);
            }

            if self.renaming_task_id == Some(task.id) {
                ui.text_edit_singleline(&mut self.rename_buf);
                if ui.small_button("OK").clicked() {
                    if let Ok(db) = self.state.db.lock() {
                        let _ = db::rename_task(&db, task.id, self.rename_buf.trim());
                    }
                    self.renaming_task_id = None;
                }
                if ui.small_button("X").clicked() {
                    self.renaming_task_id = None;
                }
            } else {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .small_button("\u{2795}")
                        .on_hover_text("Add child task")
                        .clicked()
                    {
                        if let Ok(db) = self.state.db.lock() {
                            let child_id =
                                db::create_task(&db, Some(task.id), "new task", false, true);
                            if let Ok(id) = child_id {
                                self.rename_buf = String::new();
                                self.renaming_task_id = Some(id);
                            }
                        }
                    }
                    if ui
                        .small_button("\u{1F589}")
                        .on_hover_text("Rename")
                        .clicked()
                    {
                        self.rename_buf = task.name.clone();
                        self.renaming_task_id = Some(task.id);
                    }
                    if ui
                        .small_button("\u{1F5D1}")
                        .on_hover_text("Delete")
                        .clicked()
                    {
                        self.confirm_delete_task_id = Some(task.id);
                        self.rename_buf = task.name.clone();
                    }
                    if ui
                        .small_button("\u{1F4E4}")
                        .on_hover_text("Archive / Unarchive")
                        .clicked()
                    {
                        if let Ok(db) = self.state.db.lock() {
                            let _ = db::archive_task(&db, task.id, !task.is_archived);
                        }
                    }
                    if ui
                        .small_button(if task.is_payable { "$" } else { "\u{00A2}" })
                        .on_hover_text("Toggle billable")
                        .clicked()
                    {
                        if let Ok(db) = self.state.db.lock() {
                            let _ = db::set_payable(&db, task.id, !task.is_payable);
                        }
                    }
                });
            }
        });

        let children: Vec<&db::TaskRow> = all_tasks
            .iter()
            .filter(|t| t.parent_id == Some(task.id) && (self.show_archived || !t.is_archived))
            .collect();

        for child in children {
            self.render_task_node(ui, child, all_tasks, depth + 1);
        }
    }
}

impl eframe::App for ChronosApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.tray_event_rx.is_none() {
            let (tx, rx) = std::sync::mpsc::channel();
            let ctx_clone = ctx.clone();
            std::thread::spawn(move || {
                let receiver = tray_icon::menu::MenuEvent::receiver();
                while let Ok(event) = receiver.recv() {
                    let _ = tx.send(event);
                    ctx_clone.request_repaint();
                }
            });
            self.tray_event_rx = Some(rx);
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.state.window_visible = false;
            return;
        }

        self.process_tray_events();
        self.update_menu_states();
        self.state.check_lock();

        if self.needs_visibility_update {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.state.window_visible));
            self.needs_visibility_update = false;
        }

        if ctx.input(|i| i.pointer.any_down() || i.pointer.hover_pos().is_some()) {
            self.state.register_activity();
        }

        self.state.check_idle();

        ctx.input_mut(|i| {
            let ctrl = i.modifiers.ctrl;
            if ctrl && i.key_pressed(egui::Key::Space) && !i.modifiers.shift {
                match self.state.tracker_state() {
                    TrackerState::Idle | TrackerState::Paused => {
                        if let Some(tid) = self.selected_task_id {
                            let name = self
                                .state
                                .db
                                .lock()
                                .ok()
                                .and_then(|db| db::get_task(&db, tid).ok().flatten())
                                .map(|t| t.name)
                                .unwrap_or_default();
                            let _ = self.state.start_tracking(tid, &name);
                        }
                    }
                    TrackerState::Running => {
                        let _ = self.state.stop_tracking();
                    }
                }
            }
            if ctrl && i.key_pressed(egui::Key::E) {
                let path = format!("/tmp/chronos_export_{}.csv", stats::now_ts());
                let result = self
                    .state
                    .db
                    .lock()
                    .ok()
                    .map(|db| export::export_csv(&path, &db));
                match result {
                    Some(Ok(())) => {
                        self.state.last_status = format!("Exported to {path}");
                    }
                    Some(Err(e)) => {
                        self.state.last_status = format!("Export failed: {e}");
                    }
                    None => {
                        self.state.last_status = "Export failed: DB locked".to_string();
                    }
                }
                self.state.update_status();
            }
            if ctrl && i.key_pressed(egui::Key::H) {
                self.state.toggle_window();
                self.needs_visibility_update = true;
            }
            if ctrl && i.key_pressed(egui::Key::P) {
                match self.state.tracker_state() {
                    TrackerState::Running => {
                        let _ = self.state.pause_tracking();
                    }
                    TrackerState::Paused => {
                        let _ = self.state.resume_tracking();
                    }
                    _ => {}
                }
            }
            if i.key_pressed(egui::Key::F1) {
                self.show_help = !self.show_help;
            }
        });
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        if self.state.tracker_state() != TrackerState::Idle {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }

        egui::Panel::top("status_bar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let title = egui::RichText::new("Chronos")
                    .strong()
                    .size(18.0)
                    .color(egui::Color32::from_rgb(200, 170, 255));
                ui.label(title);
                ui.separator();
                let (label, color) = match self.state.tracker_state() {
                    TrackerState::Idle => ("  Idle", egui::Color32::GRAY),
                    TrackerState::Running => ("  Running", egui::Color32::GREEN),
                    TrackerState::Paused => ("  Paused", egui::Color32::GOLD),
                };
                ui.colored_label(color, label);
                if self.state.tracker_state() != TrackerState::Idle {
                    ui.separator();
                    let secs = self.state.elapsed_seconds();
                    ui.label(app::format_duration(secs));
                    if let Some(name) = &self.state.current_task_name {
                        ui.separator();
                        ui.label(name);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("?").on_hover_text("Keyboard shortcuts").clicked() {
                        self.show_help = !self.show_help;
                    }
                    if ui
                        .button("Logs")
                        .on_hover_text("Show application logs")
                        .clicked()
                    {
                        self.show_logs = !self.show_logs;
                    }
                    if ui.button("Export CSV").clicked() {
                        let path = format!("/tmp/chronos_export_{}.csv", stats::now_ts());
                        let result = self
                            .state
                            .db
                            .lock()
                            .ok()
                            .map(|db| export::export_csv(&path, &db));
                        match result {
                            Some(Ok(())) => {
                                self.state.last_status = format!("Exported to {path}");
                            }
                            Some(Err(e)) => {
                                self.state.last_status = format!("Export failed: {e}");
                            }
                            None => {
                                self.state.last_status = "Export failed: DB locked".to_string();
                            }
                        }
                        self.state.update_status();
                    }
                });
            });
        });

        if let Some(event) = self.state.idle_dialog.clone() {
            let msg = match &event {
                IdleEvent::BecameIdle(dur) => {
                    format!(
                        "Auto-paused after {} of inactivity.",
                        app::format_duration(dur.as_secs())
                    )
                }
                IdleEvent::ReturnedFromIdle => "Welcome back! Idle time was discarded.".to_string(),
            };
            egui::Panel::top("idle_dialog").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::YELLOW, "\u{23F0}");
                    ui.label(&msg);
                    if ui.button("OK").clicked() {
                        self.state.idle.acknowledge_return();
                        self.state.idle_dialog = None;
                    }
                    if matches!(event, IdleEvent::ReturnedFromIdle) {
                        if ui.button("Discard & Continue").clicked() {
                            self.state.idle.acknowledge_return();
                            self.state.idle_dialog = None;
                        }
                    }
                });
            });
        }

        egui::Panel::bottom("stats_bar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if let Ok(db) = self.state.db.lock() {
                    if let Ok(s) = stats::compute_stats(&db) {
                        stat_card(ui, "Today", s.today, egui::Color32::LIGHT_BLUE);
                        stat_card(ui, "Yesterday", s.yesterday, egui::Color32::LIGHT_GRAY);
                        stat_card(ui, "Week", s.this_week, egui::Color32::LIGHT_GREEN);
                        stat_card(ui, "Month", s.this_month, egui::Color32::LIGHT_YELLOW);
                        ui.separator();
                        ui.colored_label(
                            egui::Color32::GOLD,
                            format!(
                                "Billable: {}",
                                app::format_duration(s.billable_today as u64)
                            ),
                        );
                    }
                }
            });
        });

        egui::Panel::left("task_panel")
            .resizable(true)
            .default_size(280.0)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Tasks");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("+").clicked() {
                            self.show_add_dialog = !self.show_add_dialog;
                        }
                        ui.checkbox(&mut self.show_archived, "archived");
                    });
                });

                ui.text_edit_singleline(&mut self.search_query);

                if self.show_add_dialog {
                    ui.group(|ui| {
                        ui.label("New Task:");
                        ui.text_edit_singleline(&mut self.new_task_name);
                        ui.horizontal(|ui| {
                            if ui.button("Add Root").clicked() {
                                let name = self.new_task_name.trim().to_string();
                                if !name.is_empty() {
                                    if let Ok(db) = self.state.db.lock() {
                                        let _ = db::create_task(&db, None, &name, true, true);
                                    }
                                    self.new_task_name.clear();
                                    self.show_add_dialog = false;
                                }
                            }
                            if let Some(pid) = self.selected_task_id {
                                if ui.button("Add Sub-task").clicked() {
                                    let name = self.new_task_name.trim().to_string();
                                    if !name.is_empty() {
                                        if let Ok(db) = self.state.db.lock() {
                                            let _ =
                                                db::create_task(&db, Some(pid), &name, false, true);
                                        }
                                        self.new_task_name.clear();
                                        self.show_add_dialog = false;
                                    }
                                }
                            }
                        });
                    });
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_task_tree(ui);
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.active_tab == Tab::Tasks, "Tasks")
                    .clicked()
                {
                    self.active_tab = Tab::Tasks;
                }
                if ui
                    .selectable_label(self.active_tab == Tab::Journal, "Journal")
                    .clicked()
                {
                    self.active_tab = Tab::Journal;
                }
                if ui
                    .selectable_label(self.active_tab == Tab::Report, "Report")
                    .clicked()
                {
                    self.active_tab = Tab::Report;
                }
            });
            ui.separator();

            match self.active_tab {
                Tab::Tasks => {
                    ui.heading("Task Details");
                    if let Some(tid) = self.selected_task_id {
                        if let Ok(db) = self.state.db.lock() {
                            if let Ok(Some(t)) = db::get_task(&db, tid) {
                                ui.label(format!("Name: {}", t.name));
                                ui.label(format!(
                                    "Project: {}",
                                    if t.is_project { "Yes" } else { "No" }
                                ));
                                ui.label(format!(
                                    "Payable: {}",
                                    if t.is_payable { "Yes" } else { "No" }
                                ));
                                ui.label(format!(
                                    "Archived: {}",
                                    if t.is_archived { "Yes" } else { "No" }
                                ));
                                ui.separator();
                                ui.label("Notes:");
                                let mut notes = t.notes.clone();
                                if ui.text_edit_multiline(&mut notes).changed() {
                                    let _ = db::set_task_notes(&db, tid, &notes);
                                }
                            }
                        }
                    } else {
                        ui.label("Select a task to view details.");
                    }
                }
                Tab::Journal => {
                    ui.heading("Time Journal");

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.filter_active, "Filter");
                        if self.filter_active {
                            ui.label("From:");
                            ui.text_edit_singleline(&mut self.filter_date_from);
                            ui.label("To:");
                            ui.text_edit_singleline(&mut self.filter_date_to);
                        }
                    });

                    if let Ok(db) = self.state.db.lock() {
                        let periods = if self.filter_active {
                            let from_ts = parse_date_filter(&self.filter_date_from, 0);
                            let to_ts =
                                parse_date_filter(&self.filter_date_to, stats::now_ts() + 86400);
                            stats::get_all_periods_in_range(&db, from_ts, to_ts).unwrap_or_default()
                        } else {
                            stats::get_all_periods_ordered(&db, 100).unwrap_or_default()
                        };
                        drop(db);

                        if periods.is_empty() {
                            ui.label("No time periods recorded yet.");
                            ui.label("Select a task and click \u{25B6} to start tracking.");
                        } else {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                egui::Grid::new("journal_grid")
                                    .striped(true)
                                    .min_col_width(80.0)
                                    .show(ui, |ui| {
                                        ui.strong("Task");
                                        ui.strong("Start");
                                        ui.strong("End");
                                        ui.strong("Duration");
                                        ui.strong("Billable");
                                        ui.end_row();

                                        for (pid, task_id, begin, end, dur, paid) in &periods {
                                            let task_name = self
                                                .state
                                                .db
                                                .lock()
                                                .ok()
                                                .and_then(|db| {
                                                    db::get_task(&db, *task_id).ok().flatten()
                                                })
                                                .map(|t| t.name)
                                                .unwrap_or_else(|| format!("#{task_id}"));

                                            let start_str = util::ts_to_string(*begin);
                                            let end_str = util::ts_to_string(*end);
                                            let dur_str = app::format_duration(*dur as u64);
                                            let paid_str =
                                                if *paid { "\u{2705}" } else { "\u{274C}" };

                                            ui.label(task_name);
                                            ui.label(start_str);
                                            ui.label(end_str);
                                            ui.label(dur_str);
                                            ui.label(paid_str);
                                            if ui
                                                .small_button("\u{1F5D1}")
                                                .on_hover_text("Delete entry")
                                                .clicked()
                                            {
                                                if let Ok(db) = self.state.db.lock() {
                                                    let _ = db::delete_time_period(&db, *pid);
                                                }
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                        }
                    }
                }
                Tab::Report => {
                    ui.heading("Summary Report");
                    ui.horizontal(|ui| {
                        ui.label("From:");
                        ui.text_edit_singleline(&mut self.report_from);
                        ui.label("To:");
                        ui.text_edit_singleline(&mut self.report_to);
                    });

                    let from_ts = parse_date_filter(&self.report_from, 0);
                    let to_ts = parse_date_filter(&self.report_to, stats::now_ts() + 86400);

                    if let Ok(db) = self.state.db.lock() {
                        if let Ok(report) = stats::get_task_report(&db, from_ts, to_ts) {
                            if report.is_empty() {
                                ui.label("No time recorded in this period.");
                            } else {
                                let total_all: i64 = report.iter().map(|r| r.total_secs).sum();
                                let billable_all: i64 =
                                    report.iter().map(|r| r.billable_secs).sum();
                                ui.horizontal(|ui| {
                                    ui.label(format!(
                                        "Total: {}",
                                        app::format_duration(total_all as u64)
                                    ));
                                    ui.separator();
                                    ui.colored_label(
                                        egui::Color32::GOLD,
                                        format!(
                                            "Billable: {}",
                                            app::format_duration(billable_all as u64)
                                        ),
                                    );
                                    ui.separator();
                                    let pct = if total_all > 0 {
                                        (billable_all as f64 / total_all as f64) * 100.0
                                    } else {
                                        0.0
                                    };
                                    ui.label(format!("{:.1}% billable", pct));
                                });
                                ui.separator();
                                egui::ScrollArea::vertical().show(ui, |ui| {
                                    egui::Grid::new("report_grid")
                                        .striped(true)
                                        .min_col_width(80.0)
                                        .show(ui, |ui| {
                                            ui.strong("Task");
                                            ui.strong("Total");
                                            ui.strong("Billable");
                                            ui.strong("%");
                                            ui.end_row();

                                            for item in &report {
                                                let pct = if item.total_secs > 0 {
                                                    (item.billable_secs as f64
                                                        / item.total_secs as f64)
                                                        * 100.0
                                                } else {
                                                    0.0
                                                };
                                                ui.label(&item.task_name);
                                                ui.label(app::format_duration(
                                                    item.total_secs as u64,
                                                ));
                                                ui.label(app::format_duration(
                                                    item.billable_secs as u64,
                                                ));
                                                ui.label(format!("{:.0}%", pct));
                                                ui.end_row();
                                            }
                                        });
                                });
                            }
                        }
                    }
                }
            }
        });

        if let Some(task_id) = self.confirm_delete_task_id {
            let task_name = self.rename_buf.clone();
            egui::Window::new("Confirm Delete")
                .id("confirm_delete".into())
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Delete \"{task_name}\" and all its time entries?"));
                    ui.label("This action cannot be undone.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.confirm_delete_task_id = None;
                        }
                        if ui.button("\u{1F5D1}  Delete").clicked() {
                            if let Ok(db) = self.state.db.lock() {
                                let _ = db::delete_task(&db, task_id);
                            }
                            self.confirm_delete_task_id = None;
                        }
                    });
                });
        }

        if self.show_help {
            egui::Window::new("Keyboard Shortcuts")
                .id("help_window".into())
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Ctrl+Space  \u{2192} Start / Stop tracking");
                    ui.label("Ctrl+P      \u{2192} Pause / Resume");
                    ui.label("Ctrl+E      \u{2192} Export CSV");
                    ui.label("Ctrl+H      \u{2192} Hide / Show window");
                    ui.label("Quit via system tray menu");
                    ui.label("F1 / ?     \u{2192} Toggle this help");
                    ui.separator();
                    ui.label("Double-click a task \u{2192} Start tracking");
                    ui.label("\u{25B6} button   \u{2192} Start tracking");
                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.show_help = false;
                    }
                });
        }

        if self.show_logs {
            egui::Window::new("Application Logs")
                .id("logs_window".into())
                .default_size([600.0, 400.0])
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Clear").clicked() {
                                if let Ok(mut logs) = crate::log_buffer::get_logs().lock() {
                                    logs.clear();
                                }
                            }
                            if ui.button("Close").clicked() {
                                self.show_logs = false;
                            }
                        });
                        ui.separator();

                        let mut log_text = if let Ok(logs) = crate::log_buffer::get_logs().lock() {
                            logs.join("")
                        } else {
                            "Failed to lock logs".to_string()
                        };

                        egui::ScrollArea::vertical()
                            .max_height(350.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut log_text)
                                        .font(egui::TextStyle::Monospace)
                                        .interactive(false)
                                        .desired_width(f32::INFINITY),
                                );
                            });
                    });
                });
        }
    }
}

fn stat_card(ui: &mut egui::Ui, label: &str, secs: i64, color: egui::Color32) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).size(10.0).color(color));
        ui.strong(app::format_duration(secs as u64));
    });
    ui.separator();
}

fn parse_date_filter(s: &str, default: i64) -> i64 {
    let s = s.trim();
    if s.is_empty() {
        return default;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return default;
    }
    let y: i64 = parts[0].parse().unwrap_or(1970);
    let m: u64 = parts[1].parse().unwrap_or(1);
    let d: u64 = parts[2].parse().unwrap_or(1);

    let mut days = 0i64;
    for year in 1970..y {
        days += if util::is_leap_year(year) { 366 } else { 365 };
    }
    let month_days = if util::is_leap_year(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for i in 0..(m.saturating_sub(1)) as usize {
        days += month_days[i] as i64;
    }
    days += (d as i64).saturating_sub(1);
    days * 86400
}
