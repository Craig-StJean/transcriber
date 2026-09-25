use std::cell::Cell;
use std::rc::Rc;

use gtk4::{gio, glib};
use libadwaita as adw;
use adw::prelude::*;

use super::checks::{active_checks, CheckResult, FixAction, StatusCheck};
use crate::ui;

/// One check's widgets.
#[derive(Clone)]
struct CheckRow {
    check:      &'static StatusCheck,
    row:        adw::ActionRow,
    icon:       gtk4::Image,
    spinner:    adw::Spinner,
    fix_btn:    gtk4::Button,
    /// Bumped per run so a slow, superseded result can't overwrite a newer one.
    generation: Rc<Cell<u32>>,
}

impl CheckRow {
    /// Run the check on a worker thread — several spawn `systemctl` or
    /// `gnome-extensions`, which would otherwise freeze the window.
    fn run(&self) {
        let this = self.clone();
        let gen = self.generation.get().wrapping_add(1);
        self.generation.set(gen);
        self.spinner.set_visible(true);
        self.fix_btn.set_visible(false);
        glib::spawn_future_local(async move {
            let run = this.check.run;
            let result = gio::spawn_blocking(run)
                .await
                .unwrap_or_else(|_| Err("Check crashed".into()));
            if this.generation.get() == gen {
                this.apply(result);
            }
        });
    }

    fn apply(&self, result: CheckResult) {
        self.spinner.set_visible(false);
        match result {
            Ok(()) => {
                self.row.set_subtitle(self.check.ok_msg);
                self.icon.set_icon_name(Some("object-select-symbolic"));
                self.icon.remove_css_class("warning");
                self.icon.add_css_class("success");
                self.fix_btn.set_visible(false);
            }
            Err(msg) => {
                self.row.set_subtitle(if msg.is_empty() { self.check.fail_msg } else { &msg });
                self.icon.set_icon_name(Some("dialog-warning-symbolic"));
                self.icon.remove_css_class("success");
                self.icon.add_css_class("warning");
                self.fix_btn.set_visible(self.check.fix.is_some());
            }
        }
    }
}

/// Returns the page and a function that re-runs every check.
pub fn build(
    toast: &adw::ToastOverlay,
    view_stack: &adw::ViewStack,
) -> (adw::PreferencesPage, Rc<dyn Fn()>) {
    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder()
        .title("Setup")
        .description("Verifies installation and configuration")
        .build();

    let mut rows = Vec::new();
    for check in active_checks() {
        let row = adw::ActionRow::builder().title(check.title).build();

        let icon = gtk4::Image::builder()
            .pixel_size(16)
            .valign(gtk4::Align::Center)
            .icon_name("content-loading-symbolic")
            .build();
        row.add_prefix(&icon);

        let spinner = adw::Spinner::builder()
            .visible(false)
            .valign(gtk4::Align::Center)
            .build();
        row.add_suffix(&spinner);

        let fix_btn = gtk4::Button::builder()
            .valign(gtk4::Align::Center)
            .css_classes(vec!["suggested-action"])
            .visible(false)
            .build();
        row.add_suffix(&fix_btn);

        let cr = CheckRow {
            check,
            row: row.clone(),
            icon,
            spinner,
            fix_btn: fix_btn.clone(),
            generation: Rc::new(Cell::new(0)),
        };
        connect_fix(&cr, toast, view_stack);
        group.add(&row);
        rows.push(cr);
    }

    let recheck_group = adw::PreferencesGroup::new();
    let recheck = adw::ButtonRow::builder()
        .title("Recheck All")
        .start_icon_name("view-refresh-symbolic")
        .build();

    let refresh_all: Rc<dyn Fn()> = Rc::new(move || rows.iter().for_each(CheckRow::run));
    recheck.connect_activated({
        let r = Rc::clone(&refresh_all);
        move |_| r()
    });
    recheck_group.add(&recheck);

    page.add(&group);
    page.add(&recheck_group);
    refresh_all();
    (page, refresh_all)
}

fn connect_fix(cr: &CheckRow, toast: &adw::ToastOverlay, view_stack: &adw::ViewStack) {
    let Some(fix) = &cr.check.fix else { return };
    match fix {
        FixAction::RunCommand { btn_label, program, args } => {
            cr.fix_btn.set_label(btn_label);
            let (program, args) = (*program, *args);
            let cr2 = cr.clone();
            let toast = toast.clone();
            cr.fix_btn.connect_clicked(move |_| {
                cr2.fix_btn.set_visible(false);
                cr2.spinner.set_visible(true);
                let cr = cr2.clone();
                let toast = toast.clone();
                glib::spawn_future_local(async move {
                    let out = gio::spawn_blocking(move || {
                        std::process::Command::new(program).args(args).output()
                    })
                    .await;
                    match out {
                        Ok(Ok(o)) if o.status.success() => {}
                        Ok(Ok(o)) => {
                            let code = o.status.code().unwrap_or(-1);
                            ui::toast(&toast, &format!("Command failed (exit {code})"));
                        }
                        Ok(Err(e)) => ui::toast(&toast, &format!("Couldn't run {program}: {e}")),
                        Err(_) => ui::toast(&toast, &format!("Couldn't run {program}")),
                    }
                    // Re-check either way: the row should show the real state,
                    // not assume the command's outcome.
                    cr.run();
                });
            });
        }
        FixAction::CopyText { btn_label, text } => {
            cr.fix_btn.set_label(btn_label);
            cr.fix_btn.set_tooltip_text(Some(&format!("Copy: {text}")));
            let text = *text;
            let toast = toast.clone();
            cr.fix_btn.connect_clicked(move |_| ui::copy_to_clipboard(&toast, text));
        }
        FixAction::GoToSettings => {
            cr.fix_btn.set_label("Go to Settings");
            let vs = view_stack.clone();
            cr.fix_btn.connect_clicked(move |_| vs.set_visible_child_name("settings"));
        }
    }
}
