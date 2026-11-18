//! Persistence between launches — the native stand-in for the mock's localStorage
//! (forge-theme, forge-pixel-scene, forge-collapsed, forge-groupby, forge-projects,
//! forge-labels, forge-ctx and the settings keys). Stored as JSON in
//! `~/.config/tako/state.json`, written (throttled) whenever it changes.

use crate::{app::TakoApp, data, sidebar::LabelDef, theme::ThemeState};
use gpui_kit::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Serialize, Deserialize, Default, PartialEq, Clone)]
#[serde(default)]
pub struct Saved {
    pub pal: String,
    pub light: bool,
    pub scene: String,
    pub sidebar_open: bool,
    pub right_open: bool,
    pub collapsed: Vec<String>,
    pub group_by_label: bool,
    pub labels: Vec<(String, String, u32)>,
    pub projects: Vec<(String, String, String, Option<String>)>,
    pub ctx: BTreeMap<String, String>,
    pub startup: String,
    pub bools: BTreeMap<String, bool>,
    pub sels: BTreeMap<String, String>,
    pub effort: String,
}

fn path() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/tako/state.json")
}

/// Snapshot runs stay deterministic: no reading or writing state.
fn enabled() -> bool {
    std::env::var("TAKO_SNAP").is_err() && std::env::var("TAKO_NO_PERSIST").is_err()
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// Find `v` among `options` so `&'static str` fields keep pointing at constants.
fn pick(options: &[&'static str], v: &str) -> Option<&'static str> {
    options.iter().copied().find(|o| *o == v)
}

impl TakoApp {
    pub fn saved_state(&self, cx: &App) -> Saved {
        let ts = cx.global::<ThemeState>();
        let s = &self.pages.settings;
        Saved {
            pal: ts.pal.clone(),
            light: ts.light,
            scene: self.scene.id.to_string(),
            sidebar_open: self.sidebar_open,
            right_open: self.right_open,
            collapsed: { let mut v: Vec<String> = self.sidebar.collapsed.iter().cloned().collect(); v.sort(); v },
            group_by_label: self.sidebar.group_by_label,
            labels: self.sidebar.labels.iter().map(|l| (l.id.clone(), l.name.clone(), l.color)).collect(),
            projects: self.projects.iter().map(|p| (p.id.into(), p.name.into(), p.icon.into(), p.label.map(String::from))).collect(),
            ctx: BTreeMap::from([
                ("access".into(), self.ctx.access.into()),
                ("env".into(), self.ctx.env.into()),
                ("model".into(), self.ctx.model.into()),
                ("effort".into(), self.ctx.effort.into()),
                ("project".into(), self.ctx.project.clone()),
                ("branch".into(), self.ctx.branch.clone()),
            ]),
            startup: s.startup.into(),
            bools: s.bools.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            sels: s.sels.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
            effort: s.effort.into(),
        }
    }

    /// Load saved state (if any) into a freshly built app.
    pub fn restore_state(&mut self, cx: &mut Context<Self>) {
        if !enabled() {
            return;
        }
        let Ok(text) = std::fs::read_to_string(path()) else { return };
        let Ok(sv) = serde_json::from_str::<Saved>(&text) else { return };
        {
            let ts = cx.global_mut::<ThemeState>();
            if !sv.pal.is_empty() {
                ts.pal = sv.pal.clone();
                ts.light = sv.light;
                ts.apply();
            }
        }
        if let Some(sc) = crate::scene::SCENES.iter().find(|s| s.id == sv.scene) {
            self.scene.id = sc.id;
        }
        self.sidebar_open = sv.sidebar_open || sv == Saved::default();
        self.right_open = sv.right_open || sv == Saved::default();
        self.sidebar.collapsed = sv.collapsed.iter().cloned().collect();
        self.sidebar.group_by_label = sv.group_by_label;
        if !sv.labels.is_empty() {
            self.sidebar.labels = sv.labels.iter().map(|(id, name, c)| LabelDef { id: id.clone(), name: name.clone(), color: *c }).collect();
        }
        if !sv.projects.is_empty() {
            self.projects = sv
                .projects
                .iter()
                .map(|(id, name, icon, label)| data::Project {
                    id: leak(id),
                    name: leak(name),
                    icon: crate::pix::icon_ids().find(|i| i == icon).unwrap_or("star"),
                    label: label.as_deref().map(leak),
                })
                .collect();
        }
        let c = &sv.ctx;
        if let Some(v) = c.get("access").and_then(|v| crate::ctx::ACCESS.iter().find(|a| a.id == v)) {
            self.ctx.access = v.id;
        }
        if let Some(v) = c.get("env") {
            self.ctx.env = crate::ctx::env(v).id;
        }
        if let Some(v) = c.get("model").and_then(|v| crate::ctx::MODELS.iter().find(|m| m.0 == v)) {
            self.ctx.model = v.0;
        }
        if let Some(v) = c.get("effort").and_then(|v| pick(crate::ctx::EFFORT, v)) {
            self.ctx.effort = v;
        }
        if let Some(v) = c.get("project") {
            self.ctx.project = v.clone();
        }
        if let Some(v) = c.get("branch") {
            self.ctx.branch = v.clone();
        }
        let s = &mut self.pages.settings;
        if let Some(v) = pick(&["home", "chats"], &sv.startup) {
            s.startup = v;
        }
        for (k, v) in &sv.bools {
            if let Some(slot) = s.bools.iter_mut().find(|(kk, _)| **kk == k.as_str()) {
                *slot.1 = *v;
            }
        }
        for (k, v) in &sv.sels {
            if let Some(slot) = s.sels.iter_mut().find(|(kk, _)| **kk == k.as_str()) {
                *slot.1 = v.clone();
            }
        }
        if let Some(v) = pick(crate::ctx::EFFORT, &sv.effort) {
            s.effort = v;
        }
        self.persist_last = Some(sv);
    }

    /// Called every render; writes at most every 400ms and only when something changed.
    pub fn persist_tick(&mut self, cx: &mut Context<Self>) {
        if !enabled() {
            return;
        }
        if self.persist_at.is_some_and(|t| t.elapsed().as_millis() < 400) {
            return;
        }
        self.persist_at = Some(std::time::Instant::now());
        let now = self.saved_state(cx);
        if self.persist_last.as_ref() == Some(&now) {
            return;
        }
        let p = path();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(&now) {
            let _ = std::fs::write(&p, json);
        }
        self.persist_last = Some(now);
    }
}
